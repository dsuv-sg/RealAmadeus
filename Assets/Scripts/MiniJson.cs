using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

/// <summary>
/// Minimal JSON parser/serializer for localization use cases.
/// Avoids the dependency on UnityWebRequest/Newtonsoft while still
/// handling nested objects, arrays, strings, numbers, booleans and null.
/// Based on the public-domain MiniJSON by Calvin Rien (2010).
/// </summary>
public static class MiniJson
{
    public static object Deserialize(string json)
    {
        if (string.IsNullOrEmpty(json)) return null;
        return Parser.Parse(json);
    }

    public static string Serialize(object obj)
    {
        return Serializer.Serialize(obj);
    }

    private enum TokenType
    {
        None, CurlyOpen, CurlyClose, SquaredOpen, SquaredClose,
        Colon, Comma, String, Number, True, False, Null
    }

    private sealed class Parser
    {
        private readonly string json;
        private int index;

        private Parser(string json)
        {
            this.json = json;
            this.index = 0;
        }

        public static object Parse(string json)
        {
            if (json == null) return null;
            return new Parser(json).ParseValue();
        }

        private void SkipWhiteSpace()
        {
            while (index < json.Length && char.IsWhiteSpace(json[index]))
            {
                index++;
            }
        }

        private object ParseValue()
        {
            SkipWhiteSpace();
            if (index >= json.Length) return null;

            char c = json[index];
            if (c == '"') return ParseString();
            if (c == '{') return ParseObject();
            if (c == '[') return ParseArray();
            if (c == 't') { ConsumeWord("true"); return true; }
            if (c == 'f') { ConsumeWord("false"); return false; }
            if (c == 'n') { ConsumeWord("null"); return null; }

            if (char.IsDigit(c) || c == '-' || c == '+')
            {
                return ParseNumber();
            }

            return null;
        }

        private void ConsumeWord(string word)
        {
            for (int i = 0; i < word.Length; i++)
            {
                if (index < json.Length && json[index] == word[i])
                {
                    index++;
                }
                else
                {
                    break;
                }
            }
        }

        private string ParseString()
        {
            index++; // consume "
            var sb = new StringBuilder();
            while (index < json.Length)
            {
                char c = json[index++];
                if (c == '"') return sb.ToString();
                if (c == '\\')
                {
                    if (index >= json.Length) break;
                    char esc = json[index++];
                    switch (esc)
                    {
                        case '"': sb.Append('"'); break;
                        case '\\': sb.Append('\\'); break;
                        case '/': sb.Append('/'); break;
                        case 'b': sb.Append('\b'); break;
                        case 'f': sb.Append('\f'); break;
                        case 'n': sb.Append('\n'); break;
                        case 'r': sb.Append('\r'); break;
                        case 't': sb.Append('\t'); break;
                        case 'u':
                            if (index + 4 <= json.Length)
                            {
                                string hex = json.Substring(index, 4);
                                index += 4;
                                sb.Append((char)Convert.ToInt32(hex, 16));
                            }
                            break;
                        default:
                            sb.Append(esc);
                            break;
                    }
                }
                else
                {
                    sb.Append(c);
                }
            }
            return sb.ToString();
        }

        private Dictionary<string, object> ParseObject()
        {
            index++; // consume {
            var dict = new Dictionary<string, object>();

            while (index < json.Length)
            {
                SkipWhiteSpace();
                if (index >= json.Length) return dict;

                char c = json[index];
                if (c == '}')
                {
                    index++;
                    return dict;
                }

                if (c == ',')
                {
                    index++;
                    continue;
                }

                string key = ParseValue() as string;
                if (key == null) return dict;

                SkipWhiteSpace();
                if (index < json.Length && json[index] == ':')
                {
                    index++;
                }
                else
                {
                    return dict;
                }

                dict[key] = ParseValue();
            }
            return dict;
        }

        private List<object> ParseArray()
        {
            index++; // consume [
            var list = new List<object>();

            while (index < json.Length)
            {
                SkipWhiteSpace();
                if (index >= json.Length) return list;

                char c = json[index];
                if (c == ']')
                {
                    index++;
                    return list;
                }

                if (c == ',')
                {
                    index++;
                    continue;
                }

                list.Add(ParseValue());
            }
            return list;
        }

        private object ParseNumber()
        {
            int start = index;
            if (index < json.Length && (json[index] == '-' || json[index] == '+'))
            {
                index++;
            }
            while (index < json.Length && (char.IsDigit(json[index]) || json[index] == '.' || json[index] == 'e' || json[index] == 'E' || json[index] == '-' || json[index] == '+'))
            {
                index++;
            }
            string s = json.Substring(start, index - start);
            if (s.Contains(".") || s.Contains("e") || s.Contains("E"))
            {
                if (double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out double d)) return d;
            }
            if (long.TryParse(s, NumberStyles.Integer, CultureInfo.InvariantCulture, out long l)) return l;
            return 0;
        }
    }

    private sealed class Serializer
    {
        private readonly StringBuilder _sb = new StringBuilder();

        public static string Serialize(object obj)
        {
            var s = new Serializer();
            s.SerializeValue(obj);
            return s._sb.ToString();
        }

        private void SerializeValue(object value)
        {
            if (value == null) { _sb.Append("null"); return; }
            if (value is string s) { SerializeString(s); return; }
            if (value is bool b) { _sb.Append(b ? "true" : "false"); return; }
            if (value is IDictionary<string, object> dict) { SerializeObject(dict); return; }
            if (value is IDictionary<string, string> sdict) { SerializeStringDict(sdict); return; }
            if (value is System.Collections.IEnumerable list) { SerializeArray(list); return; }
            if (value is double || value is float || value is int || value is long || value is short || value is byte)
            {
                _sb.Append(Convert.ToString(value, CultureInfo.InvariantCulture));
                return;
            }
            SerializeString(value.ToString());
        }

        private void SerializeObject(IDictionary<string, object> dict)
        {
            _sb.Append('{');
            bool first = true;
            foreach (var kvp in dict)
            {
                if (!first) _sb.Append(',');
                SerializeString(kvp.Key);
                _sb.Append(':');
                SerializeValue(kvp.Value);
                first = false;
            }
            _sb.Append('}');
        }

        private void SerializeStringDict(IDictionary<string, string> dict)
        {
            _sb.Append('{');
            bool first = true;
            foreach (var kvp in dict)
            {
                if (!first) _sb.Append(',');
                SerializeString(kvp.Key);
                _sb.Append(':');
                SerializeString(kvp.Value);
                first = false;
            }
            _sb.Append('}');
        }

        private void SerializeArray(System.Collections.IEnumerable list)
        {
            _sb.Append('[');
            bool first = true;
            foreach (var item in list)
            {
                if (!first) _sb.Append(',');
                SerializeValue(item);
                first = false;
            }
            _sb.Append(']');
        }

        private void SerializeString(string s)
        {
            _sb.Append('"');
            foreach (char c in s)
            {
                switch (c)
                {
                    case '"': _sb.Append("\\\""); break;
                    case '\\': _sb.Append("\\\\"); break;
                    case '\b': _sb.Append("\\b"); break;
                    case '\f': _sb.Append("\\f"); break;
                    case '\n': _sb.Append("\\n"); break;
                    case '\r': _sb.Append("\\r"); break;
                    case '\t': _sb.Append("\\t"); break;
                    default:
                        if (c < ' ') _sb.AppendFormat("\\u{0:X4}", (int)c);
                        else _sb.Append(c);
                        break;
                }
            }
            _sb.Append('"');
        }
    }
}
