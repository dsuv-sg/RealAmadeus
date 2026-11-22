"use strict";
(() => {
  var __defProp = Object.defineProperty;
  var __defProps = Object.defineProperties;
  var __getOwnPropDescs = Object.getOwnPropertyDescriptors;
  var __getOwnPropSymbols = Object.getOwnPropertySymbols;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __propIsEnum = Object.prototype.propertyIsEnumerable;
  var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
  var __spreadValues = (a, b) => {
    for (var prop in b || (b = {}))
      if (__hasOwnProp.call(b, prop))
        __defNormalProp(a, prop, b[prop]);
    if (__getOwnPropSymbols)
      for (var prop of __getOwnPropSymbols(b)) {
        if (__propIsEnum.call(b, prop))
          __defNormalProp(a, prop, b[prop]);
      }
    return a;
  };
  var __spreadProps = (a, b) => __defProps(a, __getOwnPropDescs(b));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/id/cubismid.ts
  var CubismId = class _CubismId {
    /**
     * 内部で使用するCubismIdクラス生成メソッド
     *
     * @param id ID文字列
     * @return CubismId
     * @note 指定したID文字列からCubismIdを取得する際は
     *       CubismIdManager().getId(id)を使用してください
     */
    static createIdInternal(id) {
      return new _CubismId(id);
    }
    /**
     * ID名を取得する
     */
    getString() {
      return this._id;
    }
    /**
     * idを比較
     * @param c 比較するid
     * @return 同じならばtrue,異なっていればfalseを返す
     */
    isEqual(c) {
      if (typeof c === "string") {
        return this._id == c;
      } else if (c instanceof _CubismId) {
        return this._id == c._id;
      }
      return false;
    }
    /**
     * idを比較
     * @param c 比較するid
     * @return 同じならばtrue,異なっていればfalseを返す
     */
    isNotEqual(c) {
      if (typeof c == "string") {
        return !(this._id == c);
      } else if (c instanceof _CubismId) {
        return !(this._id == c._id);
      }
      return false;
    }
    /**
     * プライベートコンストラクタ
     *
     * @note ユーザーによる生成は許可しません
     */
    constructor(id) {
      this._id = id;
    }
    // ID名
  };
  var Live2DCubismFramework;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismId = CubismId;
  })(Live2DCubismFramework || (Live2DCubismFramework = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/id/cubismidmanager.ts
  var CubismIdManager = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._ids = new Array();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      for (let i = 0; i < this._ids.length; ++i) {
        this._ids[i] = void 0;
      }
      this._ids = null;
    }
    /**
     * ID名をリストから登録
     *
     * @param ids ID名リスト
     * @param count IDの個数
     */
    registerIds(ids) {
      for (let i = 0; i < ids.length; i++) {
        this.registerId(ids[i]);
      }
    }
    /**
     * ID名を登録
     *
     * @param id ID名
     */
    registerId(id) {
      let result = null;
      if ("string" == typeof id) {
        if ((result = this.findId(id)) != null) {
          return result;
        }
        result = CubismId.createIdInternal(id);
        this._ids.push(result);
      } else {
        return this.registerId(id);
      }
      return result;
    }
    /**
     * ID名からIDを取得する
     *
     * @param id ID名
     */
    getId(id) {
      return this.registerId(id);
    }
    /**
     * ID名からIDの確認
     *
     * @return true 存在する
     * @return false 存在しない
     */
    isExist(id) {
      if ("string" == typeof id) {
        return this.findId(id) != null;
      }
      return this.isExist(id);
    }
    /**
     * ID名からIDを検索する。
     *
     * @param id ID名
     * @return 登録されているID。なければNULL。
     */
    findId(id) {
      for (let i = 0; i < this._ids.length; ++i) {
        if (this._ids[i].getString() == id) {
          return this._ids[i];
        }
      }
      return null;
    }
    // 登録されているIDのリスト
  };
  var Live2DCubismFramework2;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismIdManager = CubismIdManager;
  })(Live2DCubismFramework2 || (Live2DCubismFramework2 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/math/cubismvector2.ts
  var CubismVector2 = class _CubismVector2 {
    /**
     * コンストラクタ
     */
    constructor(x, y) {
      this.x = x;
      this.y = y;
      this.x = x == void 0 ? 0 : x;
      this.y = y == void 0 ? 0 : y;
    }
    /**
     * ベクトルの加算
     *
     * @param vector2 加算するベクトル値
     * @return 加算結果 ベクトル値
     */
    add(vector2) {
      const ret = new _CubismVector2(0, 0);
      ret.x = this.x + vector2.x;
      ret.y = this.y + vector2.y;
      return ret;
    }
    /**
     * ベクトルの減算
     *
     * @param vector2 減算するベクトル値
     * @return 減算結果 ベクトル値
     */
    substract(vector2) {
      const ret = new _CubismVector2(0, 0);
      ret.x = this.x - vector2.x;
      ret.y = this.y - vector2.y;
      return ret;
    }
    /**
     * ベクトルの乗算
     *
     * @param vector2 乗算するベクトル値
     * @return 乗算結果 ベクトル値
     */
    multiply(vector2) {
      const ret = new _CubismVector2(0, 0);
      ret.x = this.x * vector2.x;
      ret.y = this.y * vector2.y;
      return ret;
    }
    /**
     * ベクトルの乗算(スカラー)
     *
     * @param scalar 乗算するスカラー値
     * @return 乗算結果 ベクトル値
     */
    multiplyByScaler(scalar) {
      return this.multiply(new _CubismVector2(scalar, scalar));
    }
    /**
     * ベクトルの除算
     *
     * @param vector2 除算するベクトル値
     * @return 除算結果 ベクトル値
     */
    division(vector2) {
      const ret = new _CubismVector2(0, 0);
      ret.x = this.x / vector2.x;
      ret.y = this.y / vector2.y;
      return ret;
    }
    /**
     * ベクトルの除算(スカラー)
     *
     * @param scalar 除算するスカラー値
     * @return 除算結果 ベクトル値
     */
    divisionByScalar(scalar) {
      return this.division(new _CubismVector2(scalar, scalar));
    }
    /**
     * ベクトルの長さを取得する
     *
     * @return ベクトルの長さ
     */
    getLength() {
      return Math.sqrt(this.x * this.x + this.y * this.y);
    }
    /**
     * ベクトルの距離の取得
     *
     * @param a 点
     * @return ベクトルの距離
     */
    getDistanceWith(a) {
      return Math.sqrt(
        (this.x - a.x) * (this.x - a.x) + (this.y - a.y) * (this.y - a.y)
      );
    }
    /**
     * ドット積の計算
     *
     * @param a 値
     * @return 結果
     */
    dot(a) {
      return this.x * a.x + this.y * a.y;
    }
    /**
     * 正規化の適用
     */
    normalize() {
      const length = Math.pow(this.x * this.x + this.y * this.y, 0.5);
      this.x = this.x / length;
      this.y = this.y / length;
    }
    /**
     * 等しさの確認（等しいか？）
     *
     * 値が等しいか？
     *
     * @param rhs 確認する値
     * @return true 値は等しい
     * @return false 値は等しくない
     */
    isEqual(rhs) {
      return this.x == rhs.x && this.y == rhs.y;
    }
    /**
     * 等しさの確認（等しくないか？）
     *
     * 値が等しくないか？
     *
     * @param rhs 確認する値
     * @return true 値は等しくない
     * @return false 値は等しい
     */
    isNotEqual(rhs) {
      return !this.isEqual(rhs);
    }
  };
  var Live2DCubismFramework3;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismVector2 = CubismVector2;
  })(Live2DCubismFramework3 || (Live2DCubismFramework3 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/math/cubismmath.ts
  var _CubismMath = class _CubismMath {
    /**
     * 第一引数の値を最小値と最大値の範囲に収めた値を返す
     *
     * @param value 収められる値
     * @param min   範囲の最小値
     * @param max   範囲の最大値
     * @return 最小値と最大値の範囲に収めた値
     */
    static range(value, min, max) {
      if (value < min) {
        value = min;
      } else if (value > max) {
        value = max;
      }
      return value;
    }
    /**
     * サイン関数の値を求める
     *
     * @param x 角度値（ラジアン）
     * @return サイン関数sin(x)の値
     */
    static sin(x) {
      return Math.sin(x);
    }
    /**
     * コサイン関数の値を求める
     *
     * @param x 角度値(ラジアン)
     * @return コサイン関数cos(x)の値
     */
    static cos(x) {
      return Math.cos(x);
    }
    /**
     * 値の絶対値を求める
     *
     * @param x 絶対値を求める値
     * @return 値の絶対値
     */
    static abs(x) {
      return Math.abs(x);
    }
    /**
     * 平方根(ルート)を求める
     * @param x -> 平方根を求める値
     * @return 値の平方根
     */
    static sqrt(x) {
      return Math.sqrt(x);
    }
    /**
     * 立方根を求める
     * @param x -> 立方根を求める値
     * @return 値の立方根
     */
    static cbrt(x) {
      if (x === 0) {
        return x;
      }
      let cx = x;
      const isNegativeNumber = cx < 0;
      if (isNegativeNumber) {
        cx = -cx;
      }
      let ret;
      if (cx === Infinity) {
        ret = Infinity;
      } else {
        ret = Math.exp(Math.log(cx) / 3);
        ret = (cx / (ret * ret) + 2 * ret) / 3;
      }
      return isNegativeNumber ? -ret : ret;
    }
    /**
     * イージング処理されたサインを求める
     * フェードイン・アウト時のイージングに利用できる
     *
     * @param value イージングを行う値
     * @return イージング処理されたサイン値
     */
    static getEasingSine(value) {
      if (value < 0) {
        return 0;
      } else if (value > 1) {
        return 1;
      }
      return 0.5 - 0.5 * this.cos(value * Math.PI);
    }
    /**
     * 大きい方の値を返す
     *
     * @param left 左辺の値
     * @param right 右辺の値
     * @return 大きい方の値
     */
    static max(left, right) {
      return left > right ? left : right;
    }
    /**
     * 小さい方の値を返す
     *
     * @param left  左辺の値
     * @param right 右辺の値
     * @return 小さい方の値
     */
    static min(left, right) {
      return left > right ? right : left;
    }
    static clamp(val, min, max) {
      if (val < min) {
        return min;
      } else if (max < val) {
        return max;
      }
      return val;
    }
    /**
     * 角度値をラジアン値に変換する
     *
     * @param degrees   角度値
     * @return 角度値から変換したラジアン値
     */
    static degreesToRadian(degrees) {
      return degrees / 180 * Math.PI;
    }
    /**
     * ラジアン値を角度値に変換する
     *
     * @param radian    ラジアン値
     * @return ラジアン値から変換した角度値
     */
    static radianToDegrees(radian) {
      return radian * 180 / Math.PI;
    }
    /**
     * ２つのベクトルからラジアン値を求める
     *
     * @param from  始点ベクトル
     * @param to    終点ベクトル
     * @return ラジアン値から求めた方向ベクトル
     */
    static directionToRadian(from, to) {
      const q1 = Math.atan2(to.y, to.x);
      const q2 = Math.atan2(from.y, from.x);
      let ret = q1 - q2;
      while (ret < -Math.PI) {
        ret += Math.PI * 2;
      }
      while (ret > Math.PI) {
        ret -= Math.PI * 2;
      }
      return ret;
    }
    /**
     * ２つのベクトルから角度値を求める
     *
     * @param from  始点ベクトル
     * @param to    終点ベクトル
     * @return 角度値から求めた方向ベクトル
     */
    static directionToDegrees(from, to) {
      const radian = this.directionToRadian(from, to);
      let degree = this.radianToDegrees(radian);
      if (to.x - from.x > 0) {
        degree = -degree;
      }
      return degree;
    }
    /**
     * ラジアン値を方向ベクトルに変換する。
     *
     * @param totalAngle    ラジアン値
     * @return ラジアン値から変換した方向ベクトル
     */
    static radianToDirection(totalAngle) {
      const ret = new CubismVector2();
      ret.x = this.sin(totalAngle);
      ret.y = this.cos(totalAngle);
      return ret;
    }
    /**
     * 三次方程式の三次項の係数が0になったときに補欠的に二次方程式の解をもとめる。
     * a * x^2 + b * x + c = 0
     *
     * @param   a -> 二次項の係数値
     * @param   b -> 一次項の係数値
     * @param   c -> 定数項の値
     * @return  二次方程式の解
     */
    static quadraticEquation(a, b, c) {
      if (this.abs(a) < _CubismMath.Epsilon) {
        if (this.abs(b) < _CubismMath.Epsilon) {
          return -c;
        }
        return -c / b;
      }
      return -(b + this.sqrt(b * b - 4 * a * c)) / (2 * a);
    }
    /**
     * カルダノの公式によってベジェのt値に該当する３次方程式の解を求める。
     * 重解になったときには0.0～1.0の値になる解を返す。
     *
     * a * x^3 + b * x^2 + c * x + d = 0
     *
     * @param   a -> 三次項の係数値
     * @param   b -> 二次項の係数値
     * @param   c -> 一次項の係数値
     * @param   d -> 定数項の値
     * @return  0.0～1.0の間にある解
     */
    static cardanoAlgorithmForBezier(a, b, c, d) {
      if (this.abs(a) < _CubismMath.Epsilon) {
        return this.range(this.quadraticEquation(b, c, d), 0, 1);
      }
      const ba = b / a;
      const ca = c / a;
      const da = d / a;
      const p = (3 * ca - ba * ba) / 3;
      const p3 = p / 3;
      const q = (2 * ba * ba * ba - 9 * ba * ca + 27 * da) / 27;
      const q2 = q / 2;
      const discriminant = q2 * q2 + p3 * p3 * p3;
      const center = 0.5;
      const threshold = center + 0.01;
      if (discriminant < 0) {
        const mp3 = -p / 3;
        const mp33 = mp3 * mp3 * mp3;
        const r = this.sqrt(mp33);
        const t = -q / (2 * r);
        const cosphi = this.range(t, -1, 1);
        const phi = Math.acos(cosphi);
        const crtr = this.cbrt(r);
        const t1 = 2 * crtr;
        const root12 = t1 * this.cos(phi / 3) - ba / 3;
        if (this.abs(root12 - center) < threshold) {
          return this.range(root12, 0, 1);
        }
        const root2 = t1 * this.cos((phi + 2 * Math.PI) / 3) - ba / 3;
        if (this.abs(root2 - center) < threshold) {
          return this.range(root2, 0, 1);
        }
        const root3 = t1 * this.cos((phi + 4 * Math.PI) / 3) - ba / 3;
        return this.range(root3, 0, 1);
      }
      if (discriminant == 0) {
        let u12;
        if (q2 < 0) {
          u12 = this.cbrt(-q2);
        } else {
          u12 = -this.cbrt(q2);
        }
        const root12 = 2 * u12 - ba / 3;
        if (this.abs(root12 - center) < threshold) {
          return this.range(root12, 0, 1);
        }
        const root2 = -u12 - ba / 3;
        return this.range(root2, 0, 1);
      }
      const sd = this.sqrt(discriminant);
      const u1 = this.cbrt(sd - q2);
      const v1 = this.cbrt(sd + q2);
      const root1 = u1 - v1 - ba / 3;
      return this.range(root1, 0, 1);
    }
    /**
     * 浮動小数点の余りを求める。
     *
     * @param dividend 被除数（割られる値）
     * @param divisor 除数（割る値）
     * @return 余り
     */
    static mod(dividend, divisor) {
      if (!isFinite(dividend) || divisor === 0 || isNaN(dividend) || isNaN(divisor)) {
        console.warn(
          `divided: ${dividend}, divisor: ${divisor} mod() returns 'NaN'.`
        );
        return NaN;
      }
      const absDividend = Math.abs(dividend);
      const absDivisor = Math.abs(divisor);
      let result = absDividend - Math.floor(absDividend / absDivisor) * absDivisor;
      result *= Math.sign(dividend);
      return result;
    }
    /**
     * コンストラクタ
     */
    constructor() {
    }
  };
  _CubismMath.Epsilon = 1e-5;
  var CubismMath = _CubismMath;
  var Live2DCubismFramework4;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMath = CubismMath;
  })(Live2DCubismFramework4 || (Live2DCubismFramework4 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/math/cubismmatrix44.ts
  var CubismMatrix44 = class _CubismMatrix44 {
    /**
     * コンストラクタ
     */
    constructor() {
      this._tr = new Float32Array(16);
      this.loadIdentity();
    }
    /**
     * 受け取った２つの行列の乗算を行う。
     *
     * @param a 行列a
     * @param b 行列b
     *
     * @return 乗算結果の行列
     */
    static multiply(a, b, dst) {
      const c = new Float32Array([
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0
      ]);
      const n = 4;
      for (let i = 0; i < n; ++i) {
        for (let j = 0; j < n; ++j) {
          for (let k = 0; k < n; ++k) {
            c[j + i * 4] += a[k + i * 4] * b[j + k * 4];
          }
        }
      }
      for (let i = 0; i < 16; ++i) {
        dst[i] = c[i];
      }
    }
    /**
     * 単位行列に初期化する
     */
    loadIdentity() {
      const c = new Float32Array([
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1
      ]);
      this.setMatrix(c);
    }
    /**
     * 行列を設定
     *
     * @param tr 16個の浮動小数点数で表される4x4の行列
     */
    setMatrix(tr) {
      for (let i = 0; i < 16; ++i) {
        this._tr[i] = tr[i];
      }
    }
    /**
     * 行列を浮動小数点数の配列で取得
     *
     * @return 16個の浮動小数点数で表される4x4の行列
     */
    getArray() {
      return this._tr;
    }
    /**
     * X軸の拡大率を取得
     *
     * @return X軸の拡大率
     */
    getScaleX() {
      return this._tr[0];
    }
    /**
     * Y軸の拡大率を取得する
     *
     * @return Y軸の拡大率
     */
    getScaleY() {
      return this._tr[5];
    }
    /**
     * X軸の移動量を取得
     *
     * @return X軸の移動量
     */
    getTranslateX() {
      return this._tr[12];
    }
    /**
     * Y軸の移動量を取得
     *
     * @return Y軸の移動量
     */
    getTranslateY() {
      return this._tr[13];
    }
    /**
     * X軸の値を現在の行列で計算
     *
     * @param src X軸の値
     *
     * @return 現在の行列で計算されたX軸の値
     */
    transformX(src) {
      return this._tr[0] * src + this._tr[12];
    }
    /**
     * Y軸の値を現在の行列で計算
     *
     * @param src Y軸の値
     *
     * @return 現在の行列で計算されたY軸の値
     */
    transformY(src) {
      return this._tr[5] * src + this._tr[13];
    }
    /**
     * X軸の値を現在の行列で逆計算
     */
    invertTransformX(src) {
      return (src - this._tr[12]) / this._tr[0];
    }
    /**
     * Y軸の値を現在の行列で逆計算
     */
    invertTransformY(src) {
      return (src - this._tr[13]) / this._tr[5];
    }
    /**
     * 現在の行列の位置を起点にして移動
     *
     * 現在の行列の位置を起点にして相対的に移動する。
     *
     * @param x X軸の移動量
     * @param y Y軸の移動量
     */
    translateRelative(x, y) {
      const tr1 = new Float32Array([
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        x,
        y,
        0,
        1
      ]);
      _CubismMatrix44.multiply(tr1, this._tr, this._tr);
    }
    /**
     * 現在の行列の位置を移動
     *
     * 現在の行列の位置を指定した位置へ移動する
     *
     * @param x X軸の移動量
     * @param y y軸の移動量
     */
    translate(x, y) {
      this._tr[12] = x;
      this._tr[13] = y;
    }
    /**
     * 現在の行列のX軸の位置を指定した位置へ移動する
     *
     * @param x X軸の移動量
     */
    translateX(x) {
      this._tr[12] = x;
    }
    /**
     * 現在の行列のY軸の位置を指定した位置へ移動する
     *
     * @param y Y軸の移動量
     */
    translateY(y) {
      this._tr[13] = y;
    }
    /**
     * 現在の行列の拡大率を相対的に設定する
     *
     * @param x X軸の拡大率
     * @param y Y軸の拡大率
     */
    scaleRelative(x, y) {
      const tr1 = new Float32Array([
        x,
        0,
        0,
        0,
        0,
        y,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1
      ]);
      _CubismMatrix44.multiply(tr1, this._tr, this._tr);
    }
    /**
     * 現在の行列の拡大率を指定した倍率に設定する
     *
     * @param x X軸の拡大率
     * @param y Y軸の拡大率
     */
    scale(x, y) {
      this._tr[0] = x;
      this._tr[5] = y;
    }
    /**
     * 引数で与えられた行列にこの行列を乗算する。
     * (引数で与えられた行列) * (この行列)
     *
     * @note 関数名と実際の計算内容に乖離があるため、今後計算順が修正される可能性があります。
     * @param m 行列
     */
    multiplyByMatrix(m) {
      _CubismMatrix44.multiply(m.getArray(), this._tr, this._tr);
    }
    /**
     * 現在の行列の逆行列を求める。
     *
     * @return 現在の行列で計算された逆行列の値を返す
     */
    getInvert() {
      const r00 = this._tr[0];
      const r10 = this._tr[1];
      const r20 = this._tr[2];
      const r01 = this._tr[4];
      const r11 = this._tr[5];
      const r21 = this._tr[6];
      const r02 = this._tr[8];
      const r12 = this._tr[9];
      const r22 = this._tr[10];
      const tx = this._tr[12];
      const ty = this._tr[13];
      const tz = this._tr[14];
      const det = r00 * (r11 * r22 - r12 * r21) - r01 * (r10 * r22 - r12 * r20) + r02 * (r10 * r21 - r11 * r20);
      const dst = new _CubismMatrix44();
      if (CubismMath.abs(det) < CubismMath.Epsilon) {
        dst.loadIdentity();
        return dst;
      }
      const invDet = 1 / det;
      const inv00 = (r11 * r22 - r12 * r21) * invDet;
      const inv01 = -(r01 * r22 - r02 * r21) * invDet;
      const inv02 = (r01 * r12 - r02 * r11) * invDet;
      const inv10 = -(r10 * r22 - r12 * r20) * invDet;
      const inv11 = (r00 * r22 - r02 * r20) * invDet;
      const inv12 = -(r00 * r12 - r02 * r10) * invDet;
      const inv20 = (r10 * r21 - r11 * r20) * invDet;
      const inv21 = -(r00 * r21 - r01 * r20) * invDet;
      const inv22 = (r00 * r11 - r01 * r10) * invDet;
      dst._tr[0] = inv00;
      dst._tr[1] = inv10;
      dst._tr[2] = inv20;
      dst._tr[3] = 0;
      dst._tr[4] = inv01;
      dst._tr[5] = inv11;
      dst._tr[6] = inv21;
      dst._tr[7] = 0;
      dst._tr[8] = inv02;
      dst._tr[9] = inv12;
      dst._tr[10] = inv22;
      dst._tr[11] = 0;
      dst._tr[12] = -(inv00 * tx + inv01 * ty + inv02 * tz);
      dst._tr[13] = -(inv10 * tx + inv11 * ty + inv12 * tz);
      dst._tr[14] = -(inv20 * tx + inv21 * ty + inv22 * tz);
      dst._tr[15] = 1;
      return dst;
    }
    /**
     * オブジェクトのコピーを生成する
     */
    clone() {
      const cloneMatrix = new _CubismMatrix44();
      for (let i = 0; i < this._tr.length; i++) {
        cloneMatrix._tr[i] = this._tr[i];
      }
      return cloneMatrix;
    }
    // 4x4行列データ
  };
  var Live2DCubismFramework5;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMatrix44 = CubismMatrix44;
  })(Live2DCubismFramework5 || (Live2DCubismFramework5 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/type/csmrectf.ts
  var csmRect = class {
    /**
     * コンストラクタ
     * @param x 左端X座標
     * @param y 上端Y座標
     * @param w 幅
     * @param h 高さ
     */
    constructor(x, y, w, h) {
      this.x = x;
      this.y = y;
      this.width = w;
      this.height = h;
    }
    /**
     * 矩形中央のX座標を取得する
     */
    getCenterX() {
      return this.x + 0.5 * this.width;
    }
    /**
     * 矩形中央のY座標を取得する
     */
    getCenterY() {
      return this.y + 0.5 * this.height;
    }
    /**
     * 右側のX座標を取得する
     */
    getRight() {
      return this.x + this.width;
    }
    /**
     * 下端のY座標を取得する
     */
    getBottom() {
      return this.y + this.height;
    }
    /**
     * 矩形に値をセットする
     * @param r 矩形のインスタンス
     */
    setRect(r) {
      this.x = r.x;
      this.y = r.y;
      this.width = r.width;
      this.height = r.height;
    }
    /**
     * 矩形中央を軸にして縦横を拡縮する
     * @param w 幅方向に拡縮する量
     * @param h 高さ方向に拡縮する量
     */
    expand(w, h) {
      this.x -= w;
      this.y -= h;
      this.width += w * 2;
      this.height += h * 2;
    }
    // 高さ
  };
  var Live2DCubismFramework6;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.csmRect = csmRect;
  })(Live2DCubismFramework6 || (Live2DCubismFramework6 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/cubismframeworkconfig.ts
  var CSM_LOG_LEVEL_VERBOSE = 0;
  var CSM_LOG_LEVEL_DEBUG = 1;
  var CSM_LOG_LEVEL_INFO = 2;
  var CSM_LOG_LEVEL_WARNING = 3;
  var CSM_LOG_LEVEL_ERROR = 4;
  var CSM_LOG_LEVEL = CSM_LOG_LEVEL_VERBOSE;

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/utils/cubismdebug.ts
  var CubismLogPrint = (level, fmt, args) => {
    CubismDebug.print(level, "[CSM]" + fmt, args);
  };
  var CubismLogPrintIn = (level, fmt, args) => {
    CubismLogPrint(level, fmt + "\n", args);
  };
  var CSM_ASSERT = (expr) => {
    console.assert(expr);
  };
  var CubismLogVerbose;
  var CubismLogDebug;
  var CubismLogInfo;
  var CubismLogWarning;
  var CubismLogError;
  if (CSM_LOG_LEVEL <= CSM_LOG_LEVEL_VERBOSE) {
    CubismLogVerbose = (fmt, ...args) => {
      CubismLogPrintIn(0 /* LogLevel_Verbose */, "[V]" + fmt, args);
    };
    CubismLogDebug = (fmt, ...args) => {
      CubismLogPrintIn(1 /* LogLevel_Debug */, "[D]" + fmt, args);
    };
    CubismLogInfo = (fmt, ...args) => {
      CubismLogPrintIn(2 /* LogLevel_Info */, "[I]" + fmt, args);
    };
    CubismLogWarning = (fmt, ...args) => {
      CubismLogPrintIn(3 /* LogLevel_Warning */, "[W]" + fmt, args);
    };
    CubismLogError = (fmt, ...args) => {
      CubismLogPrintIn(4 /* LogLevel_Error */, "[E]" + fmt, args);
    };
  } else if (CSM_LOG_LEVEL == CSM_LOG_LEVEL_DEBUG) {
    CubismLogDebug = (fmt, ...args) => {
      CubismLogPrintIn(1 /* LogLevel_Debug */, "[D]" + fmt, args);
    };
    CubismLogInfo = (fmt, ...args) => {
      CubismLogPrintIn(2 /* LogLevel_Info */, "[I]" + fmt, args);
    };
    CubismLogWarning = (fmt, ...args) => {
      CubismLogPrintIn(3 /* LogLevel_Warning */, "[W]" + fmt, args);
    };
    CubismLogError = (fmt, ...args) => {
      CubismLogPrintIn(4 /* LogLevel_Error */, "[E]" + fmt, args);
    };
  } else if (CSM_LOG_LEVEL == CSM_LOG_LEVEL_INFO) {
    CubismLogInfo = (fmt, ...args) => {
      CubismLogPrintIn(2 /* LogLevel_Info */, "[I]" + fmt, args);
    };
    CubismLogWarning = (fmt, ...args) => {
      CubismLogPrintIn(3 /* LogLevel_Warning */, "[W]" + fmt, args);
    };
    CubismLogError = (fmt, ...args) => {
      CubismLogPrintIn(4 /* LogLevel_Error */, "[E]" + fmt, args);
    };
  } else if (CSM_LOG_LEVEL == CSM_LOG_LEVEL_WARNING) {
    CubismLogWarning = (fmt, ...args) => {
      CubismLogPrintIn(3 /* LogLevel_Warning */, "[W]" + fmt, args);
    };
    CubismLogError = (fmt, ...args) => {
      CubismLogPrintIn(4 /* LogLevel_Error */, "[E]" + fmt, args);
    };
  } else if (CSM_LOG_LEVEL == CSM_LOG_LEVEL_ERROR) {
    CubismLogError = (fmt, ...args) => {
      CubismLogPrintIn(4 /* LogLevel_Error */, "[E]" + fmt, args);
    };
  }
  var CubismDebug = class {
    /**
     * ログを出力する。第一引数にログレベルを設定する。
     * CubismFramework.initialize()時にオプションで設定されたログ出力レベルを下回る場合はログに出さない。
     *
     * @param logLevel ログレベルの設定
     * @param format 書式付き文字列
     * @param args 可変長引数
     */
    static print(logLevel, format, args) {
      if (logLevel < CubismFramework.getLoggingLevel()) {
        return;
      }
      const logPrint = CubismFramework.coreLogFunction;
      if (!logPrint) return;
      const buffer = format.replace(/\{(\d+)\}/g, (m, k) => {
        return args[k];
      });
      logPrint(buffer);
    }
    /**
     * データから指定した長さだけダンプ出力する。
     * CubismFramework.initialize()時にオプションで設定されたログ出力レベルを下回る場合はログに出さない。
     *
     * @param logLevel ログレベルの設定
     * @param data ダンプするデータ
     * @param length ダンプする長さ
     */
    static dumpBytes(logLevel, data, length) {
      for (let i = 0; i < length; i++) {
        if (i % 16 == 0 && i > 0) this.print(logLevel, "\n");
        else if (i % 8 == 0 && i > 0) this.print(logLevel, "  ");
        this.print(logLevel, "{0} ", [data[i] & 255]);
      }
      this.print(logLevel, "\n");
    }
    /**
     * private コンストラクタ
     */
    constructor() {
    }
  };
  var Live2DCubismFramework7;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismDebug = CubismDebug;
  })(Live2DCubismFramework7 || (Live2DCubismFramework7 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismrenderer.ts
  var CubismRenderer = class {
    /**
     * レンダラのインスタンスを生成して取得する
     *
     * @return レンダラのインスタンス
     */
    static create() {
      return null;
    }
    /**
     * レンダラのインスタンスを解放する
     */
    static delete(renderer) {
      renderer = null;
    }
    /**
     * レンダラの初期化処理を実行する
     * 引数に渡したモデルからレンダラの初期化処理に必要な情報を取り出すことができる
     *
     * @param model モデルのインスタンス
     */
    initialize(model) {
      this._model = model;
      if (model.isBlendModeEnabled()) {
        this.useHighPrecisionMask(true);
        CubismLogInfo(
          "This model uses a high-resolution mask because it operates in blend mode."
        );
      }
    }
    /**
     * モデルを描画する
     * @param shaderPath ブレンドモード用シェーダのパス
     */
    drawModel(shaderPath = null) {
      if (this.getModel() == null) return;
      this.doDrawModel(shaderPath);
    }
    /**
     * Model-View-Projection 行列をセットする
     * 配列は複製されるので、元の配列は外で破棄して良い
     *
     * @param matrix44 Model-View-Projection 行列
     */
    setMvpMatrix(matrix44) {
      this._mvpMatrix4x4.setMatrix(matrix44.getArray());
    }
    /**
     * Model-View-Projection 行列を取得する
     *
     * @return Model-View-Projection 行列
     */
    getMvpMatrix() {
      return this._mvpMatrix4x4;
    }
    /**
     * モデルの色をセットする
     * 各色0.0~1.0の間で指定する（1.0が標準の状態）
     *
     * @param red 赤チャンネルの値
     * @param green 緑チャンネルの値
     * @param blue 青チャンネルの値
     * @param alpha αチャンネルの値
     */
    setModelColor(red, green, blue, alpha) {
      this._modelColor.r = CubismMath.clamp(red, 0, 1);
      this._modelColor.g = CubismMath.clamp(green, 0, 1);
      this._modelColor.b = CubismMath.clamp(blue, 0, 1);
      this._modelColor.a = CubismMath.clamp(alpha, 0, 1);
    }
    /**
     * モデルの色を取得する
     * 各色0.0~1.0の間で指定する(1.0が標準の状態)
     *
     * @return RGBAのカラー情報
     */
    getModelColor() {
      return JSON.parse(JSON.stringify(this._modelColor));
    }
    /**
     * 透明度を考慮したモデルの色を計算する。
     *
     * @param opacity 透明度
     *
     * @return RGBAのカラー情報
     */
    getModelColorWithOpacity(opacity) {
      const modelColorRGBA = this.getModelColor();
      modelColorRGBA.a *= opacity;
      if (this.isPremultipliedAlpha()) {
        modelColorRGBA.r *= modelColorRGBA.a;
        modelColorRGBA.g *= modelColorRGBA.a;
        modelColorRGBA.b *= modelColorRGBA.a;
      }
      return modelColorRGBA;
    }
    /**
     * 乗算済みαの有効・無効をセットする
     * 有効にするならtrue、無効にするならfalseをセットする
     */
    setIsPremultipliedAlpha(enable) {
      this._isPremultipliedAlpha = enable;
    }
    /**
     * 乗算済みαの有効・無効を取得する
     * @return true 乗算済みのα有効
     *         false 乗算済みのα無効
     */
    isPremultipliedAlpha() {
      return this._isPremultipliedAlpha;
    }
    /**
     * カリング（片面描画）の有効・無効をセットする。
     * 有効にするならtrue、無効にするならfalseをセットする
     */
    setIsCulling(culling) {
      this._isCulling = culling;
    }
    /**
     * カリング（片面描画）の有効・無効を取得する。
     *
     * @return true カリング有効
     *         false カリング無効
     */
    isCulling() {
      return this._isCulling;
    }
    /**
     * テクスチャの異方性フィルタリングのパラメータをセットする
     * パラメータ値の影響度はレンダラの実装に依存する
     *
     * @param n パラメータの値
     */
    setAnisotropy(n) {
      this._anisotropy = n;
    }
    /**
     * テクスチャの異方性フィルタリングのパラメータをセットする
     *
     * @return 異方性フィルタリングのパラメータ
     */
    getAnisotropy() {
      return this._anisotropy;
    }
    /**
     * レンダリングするモデルを取得する
     *
     * @return レンダリングするモデル
     */
    getModel() {
      return this._model;
    }
    /**
     * マスク描画の方式を変更する。
     * falseの場合、マスクを1枚のテクスチャに分割してレンダリングする（デフォルト）
     * 高速だが、マスク個数の上限が36に限定され、質も荒くなる
     * trueの場合、パーツ描画の前にその都度必要なマスクを描き直す
     * レンダリング品質は高いが描画処理負荷は増す
     *
     * @param high 高精細マスクに切り替えるか？
     */
    useHighPrecisionMask(high) {
      this._useHighPrecisionMask = high;
    }
    /**
     * マスクの描画方式を取得する
     *
     * @return true 高精細方式
     *         false デフォルト
     */
    isUsingHighPrecisionMask() {
      return this._useHighPrecisionMask;
    }
    /**
     * モデルを描画したバッファのサイズを設定
     *
     * @param[in]   width  -> モデルを描画したバッファの幅
     * @param[in]   height -> モデルを描画したバッファの高さ
     */
    setRenderTargetSize(width, height) {
      this._modelRenderTargetWidth = width;
      this._modelRenderTargetHeight = height;
    }
    /**
     * コンストラクタ
     */
    constructor(width, height) {
      this._modelRenderTargetWidth = width;
      this._modelRenderTargetHeight = height;
      this._isCulling = false;
      this._isPremultipliedAlpha = false;
      this._anisotropy = 0;
      this._model = null;
      this._modelColor = new CubismTextureColor();
      this._useHighPrecisionMask = false;
      this._mvpMatrix4x4 = new CubismMatrix44();
      this._mvpMatrix4x4.loadIdentity();
    }
  };
  var CubismBlendMode = /* @__PURE__ */ ((CubismBlendMode2) => {
    CubismBlendMode2[CubismBlendMode2["CubismBlendMode_Normal"] = 0] = "CubismBlendMode_Normal";
    CubismBlendMode2[CubismBlendMode2["CubismBlendMode_Additive"] = 1] = "CubismBlendMode_Additive";
    CubismBlendMode2[CubismBlendMode2["CubismBlendMode_Multiplicative"] = 2] = "CubismBlendMode_Multiplicative";
    return CubismBlendMode2;
  })(CubismBlendMode || {});
  var CubismTextureColor = class {
    /**
     * コンストラクタ
     */
    constructor(r = 1, g = 1, b = 1, a = 1) {
      this.r = r;
      this.g = g;
      this.b = b;
      this.a = a;
    }
    // αチャンネル
  };
  var CubismClippingContext = class {
    /**
     * 引数付きコンストラクタ
     */
    constructor(clippingDrawableIndices, clipCount) {
      this._clippingIdList = clippingDrawableIndices;
      this._clippingIdCount = clipCount;
      this._allClippedDrawRect = new csmRect();
      this._layoutBounds = new csmRect();
      this._clippedDrawableIndexList = [];
      this._clippedOffscreenIndexList = [];
      this._matrixForMask = new CubismMatrix44();
      this._matrixForDraw = new CubismMatrix44();
      this._bufferIndex = 0;
      this._layoutChannelIndex = 0;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      if (this._layoutBounds != null) {
        this._layoutBounds = null;
      }
      if (this._allClippedDrawRect != null) {
        this._allClippedDrawRect = null;
      }
      if (this._clippedDrawableIndexList != null) {
        this._clippedDrawableIndexList = null;
      }
      if (this._clippedOffscreenIndexList != null) {
        this._clippedOffscreenIndexList = null;
      }
    }
    /**
     * このマスクにクリップされる描画オブジェクトを追加する
     *
     * @param drawableIndex クリッピング対象に追加する描画オブジェクトのインデックス
     */
    addClippedDrawable(drawableIndex) {
      this._clippedDrawableIndexList.push(drawableIndex);
    }
    /**
     * このマスクにクリップされるオフスクリーンオブジェクトを追加する
     *
     * @param offscreenIndex クリッピング対象に追加するオフスクリーンオブジェクトのインデックス
     */
    addClippedOffscreen(offscreenIndex) {
      this._clippedOffscreenIndexList.push(offscreenIndex);
    }
    // このマスクが割り当てられるレンダーテクスチャ（フレームバッファ）やカラーバッファのインデックス
  };
  var Live2DCubismFramework8;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismBlendMode = CubismBlendMode;
    Live2DCubismFramework51.CubismRenderer = CubismRenderer;
    Live2DCubismFramework51.CubismTextureColor = CubismTextureColor;
  })(Live2DCubismFramework8 || (Live2DCubismFramework8 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/utils/cubismjsonextension.ts
  var CubismJsonExtension = class _CubismJsonExtension {
    static parseJsonObject(obj, map) {
      Object.keys(obj).forEach((key) => {
        if (typeof obj[key] == "boolean") {
          const convValue = Boolean(obj[key]);
          map.put(key, new JsonBoolean(convValue));
        } else if (typeof obj[key] == "string") {
          const convValue = String(obj[key]);
          map.put(key, new JsonString(convValue));
        } else if (typeof obj[key] == "number") {
          const convValue = Number(obj[key]);
          map.put(key, new JsonFloat(convValue));
        } else if (obj[key] instanceof Array) {
          map.put(
            key,
            _CubismJsonExtension.parseJsonArray(obj[key])
          );
        } else if (obj[key] instanceof Object) {
          map.put(
            key,
            _CubismJsonExtension.parseJsonObject(obj[key], new JsonMap())
          );
        } else if (obj[key] == null) {
          map.put(key, new JsonNullvalue());
        } else {
          map.put(key, obj[key]);
        }
      });
      return map;
    }
    static parseJsonArray(obj) {
      const arr = new JsonArray();
      Object.keys(obj).forEach((key) => {
        const convKey = Number(key);
        if (typeof convKey == "number") {
          if (typeof obj[key] == "boolean") {
            const convValue = Boolean(obj[key]);
            arr.add(new JsonBoolean(convValue));
          } else if (typeof obj[key] == "string") {
            const convValue = String(obj[key]);
            arr.add(new JsonString(convValue));
          } else if (typeof obj[key] == "number") {
            const convValue = Number(obj[key]);
            arr.add(new JsonFloat(convValue));
          } else if (obj[key] instanceof Array) {
            arr.add(this.parseJsonArray(obj[key]));
          } else if (obj[key] instanceof Object) {
            arr.add(this.parseJsonObject(obj[key], new JsonMap()));
          } else if (obj[key] == null) {
            arr.add(new JsonNullvalue());
          } else {
            arr.add(obj[key]);
          }
        } else if (obj[key] instanceof Array) {
          arr.add(this.parseJsonArray(obj[key]));
        } else if (obj[key] instanceof Object) {
          arr.add(this.parseJsonObject(obj[key], new JsonMap()));
        } else if (obj[key] == null) {
          arr.add(new JsonNullvalue());
        } else {
          const convValue = Array(obj[key]);
          for (let i = 0; i < convValue.length; i++) {
            arr.add(convValue[i]);
          }
        }
      });
      return arr;
    }
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/utils/cubismjson.ts
  var CSM_JSON_ERROR_TYPE_MISMATCH = "Error: type mismatch";
  var CSM_JSON_ERROR_INDEX_OF_BOUNDS = "Error: index out of bounds";
  var Value2 = class _Value {
    /**
     * コンストラクタ
     */
    constructor() {
    }
    /**
     * 要素を文字列型で返す(string)
     */
    getRawString(defaultValue, indent) {
      return this.getString(defaultValue, indent);
    }
    /**
     * 要素を数値型で返す(number)
     */
    toInt(defaultValue = 0) {
      return defaultValue;
    }
    /**
     * 要素を数値型で返す(number)
     */
    toFloat(defaultValue = 0) {
      return defaultValue;
    }
    /**
     * 要素を真偽値で返す(boolean)
     */
    toBoolean(defaultValue = false) {
      return defaultValue;
    }
    /**
     * サイズを返す
     */
    getSize() {
      return 0;
    }
    /**
     * 要素を配列で返す(Value[])
     */
    getArray(defaultValue = null) {
      return defaultValue;
    }
    /**
     * 要素をコンテナで返す(array)
     */
    getVector(defaultValue = new Array()) {
      return defaultValue;
    }
    /**
     * 要素をマップで返す(Map<String, Value>)
     */
    getMap(defaultValue) {
      return defaultValue;
    }
    /**
     * 添字演算子[index]
     */
    getValueByIndex(index) {
      return _Value.errorValue.setErrorNotForClientCall(
        CSM_JSON_ERROR_TYPE_MISMATCH
      );
    }
    /**
     * 添字演算子[string]
     */
    getValueByString(s) {
      return _Value.nullValue.setErrorNotForClientCall(
        CSM_JSON_ERROR_TYPE_MISMATCH
      );
    }
    /**
     * マップのキー一覧をコンテナで返す
     *
     * @return マップのキーの一覧
     */
    getKeys() {
      return _Value.dummyKeys;
    }
    /**
     * Valueの種類がエラー値ならtrue
     */
    isError() {
      return false;
    }
    /**
     * Valueの種類がnullならtrue
     */
    isNull() {
      return false;
    }
    /**
     * Valueの種類が真偽値ならtrue
     */
    isBool() {
      return false;
    }
    /**
     * Valueの種類が数値型ならtrue
     */
    isFloat() {
      return false;
    }
    /**
     * Valueの種類が文字列ならtrue
     */
    isString() {
      return false;
    }
    /**
     * Valueの種類が配列ならtrue
     */
    isArray() {
      return false;
    }
    /**
     * Valueの種類がマップ型ならtrue
     */
    isMap() {
      return false;
    }
    equals(value) {
      return false;
    }
    /**
     * Valueの値が静的ならtrue、静的なら解放しない
     */
    isStatic() {
      return false;
    }
    /**
     * Valueにエラー値をセットする
     */
    setErrorNotForClientCall(errorStr) {
      return JsonError.errorValue;
    }
    /**
     * 初期化用メソッド
     */
    static staticInitializeNotForClientCall() {
      JsonBoolean.trueValue = new JsonBoolean(true);
      JsonBoolean.falseValue = new JsonBoolean(false);
      _Value.errorValue = new JsonError("ERROR", true);
      _Value.nullValue = new JsonNullvalue();
      _Value.dummyKeys = new Array();
    }
    /**
     * リリース用メソッド
     */
    static staticReleaseNotForClientCall() {
      JsonBoolean.trueValue = null;
      JsonBoolean.falseValue = null;
      _Value.errorValue = null;
      _Value.nullValue = null;
      _Value.dummyKeys = null;
    }
    // 明示的に連想配列をany型で指定
  };
  var CubismJson = class _CubismJson {
    /**
     * コンストラクタ
     */
    constructor(buffer, length) {
      this._parseCallback = CubismJsonExtension.parseJsonObject;
      this._error = null;
      this._lineCount = 0;
      this._root = null;
      if (buffer != void 0) {
        this.parseBytes(buffer, length, this._parseCallback);
      }
    }
    /**
     * バイトデータから直接ロードしてパースする
     *
     * @param buffer バッファ
     * @param size バッファサイズ
     * @return CubismJsonクラスのインスタンス。失敗したらNULL
     */
    static create(buffer, size) {
      const json = new _CubismJson();
      const succeeded = json.parseBytes(
        buffer,
        size,
        json._parseCallback
      );
      if (!succeeded) {
        _CubismJson.delete(json);
        return null;
      } else {
        return json;
      }
    }
    /**
     * パースしたJSONオブジェクトの解放処理
     *
     * @param instance CubismJsonクラスのインスタンス
     */
    static delete(instance) {
      instance = null;
    }
    /**
     * パースしたJSONのルート要素を返す
     */
    getRoot() {
      return this._root;
    }
    /**
     *  UnicodeのバイナリをStringに変換
     *
     * @param buffer 変換するバイナリデータ
     * @return 変換後の文字列
     */
    static arrayBufferToString(buffer) {
      if (typeof TextDecoder !== "undefined") {
        return new TextDecoder("utf-8").decode(buffer);
      }
      const uint8Array = new Uint8Array(buffer);
      let str = "";
      for (let i = 0, len = uint8Array.length; i < len; ++i) {
        str += "%" + this.pad(uint8Array[i].toString(16));
      }
      str = decodeURIComponent(str);
      return str;
    }
    /**
     * エンコード、パディング
     */
    static pad(n) {
      return n.length < 2 ? "0" + n : n;
    }
    /**
     * JSONのパースを実行する
     * @param buffer    パース対象のデータバイト
     * @param size      データバイトのサイズ
     * return true : 成功
     * return false: 失敗
     */
    parseBytes(buffer, size, parseCallback) {
      const endPos = new Array(1);
      const decodeBuffer = _CubismJson.arrayBufferToString(buffer);
      if (parseCallback == void 0) {
        this._root = this.parseValue(decodeBuffer, size, 0, endPos);
      } else {
        this._root = parseCallback(JSON.parse(decodeBuffer), new JsonMap());
      }
      if (this._error) {
        let strbuf = "\0";
        strbuf = "Json parse error : @line " + (this._lineCount + 1) + "\n";
        this._root = new JsonString(strbuf);
        CubismLogInfo("{0}", this._root.getRawString());
        return false;
      } else if (this._root == null) {
        this._root = new JsonError(this._error, false);
        return false;
      }
      return true;
    }
    /**
     * パース時のエラー値を返す
     */
    getParseError() {
      return this._error;
    }
    /**
     * ルート要素の次の要素がファイルの終端だったらtrueを返す
     */
    checkEndOfFile() {
      return this._root.getArray()[1].equals("EOF");
    }
    /**
     * JSONエレメントからValue(float,String,Value*,Array,null,true,false)をパースする
     * エレメントの書式に応じて内部でParseString(), ParseObject(), ParseArray()を呼ぶ
     *
     * @param   buffer      JSONエレメントのバッファ
     * @param   length      パースする長さ
     * @param   begin       パースを開始する位置
     * @param   outEndPos   パース終了時の位置
     * @return      パースから取得したValueオブジェクト
     */
    parseValue(buffer, length, begin, outEndPos) {
      if (this._error) return null;
      let o = null;
      let i = begin;
      let f;
      for (; i < length; i++) {
        const c = buffer[i];
        switch (c) {
          case "-":
          case ".":
          case "0":
          case "1":
          case "2":
          case "3":
          case "4":
          case "5":
          case "6":
          case "7":
          case "8":
          case "9": {
            const afterString = new Array(1);
            f = strtod(buffer.slice(i), afterString);
            outEndPos[0] = buffer.indexOf(afterString[0]);
            return new JsonFloat(f);
          }
          case '"':
            return new JsonString(
              this.parseString(buffer, length, i + 1, outEndPos)
            );
          // \"の次の文字から
          case "[":
            o = this.parseArray(buffer, length, i + 1, outEndPos);
            return o;
          case "{":
            o = this.parseObject(buffer, length, i + 1, outEndPos);
            return o;
          case "n":
            if (i + 3 < length) {
              o = new JsonNullvalue();
              outEndPos[0] = i + 4;
            } else {
              this._error = "parse null";
            }
            return o;
          case "t":
            if (i + 3 < length) {
              o = JsonBoolean.trueValue;
              outEndPos[0] = i + 4;
            } else {
              this._error = "parse true";
            }
            return o;
          case "f":
            if (i + 4 < length) {
              o = JsonBoolean.falseValue;
              outEndPos[0] = i + 5;
            } else {
              this._error = "illegal ',' position";
            }
            return o;
          case ",":
            this._error = "illegal ',' position";
            return null;
          case "]":
            outEndPos[0] = i;
            return null;
          case "\n":
            this._lineCount++;
          // falls through
          case " ":
          case "	":
          case "\r":
          default:
            break;
        }
      }
      this._error = "illegal end of value";
      return null;
    }
    /**
     * 次の「"」までの文字列をパースする。
     *
     * @param   string  ->  パース対象の文字列
     * @param   length  ->  パースする長さ
     * @param   begin   ->  パースを開始する位置
     * @param  outEndPos   ->  パース終了時の位置
     * @return      パースした文F字列要素
     */
    parseString(string, length, begin, outEndPos) {
      if (this._error) {
        return null;
      }
      if (!string) {
        this._error = "string is null";
        return null;
      }
      let i = begin;
      let c, c2;
      let ret = "";
      let bufStart = begin;
      for (; i < length; i++) {
        c = string[i];
        switch (c) {
          case '"': {
            outEndPos[0] = i + 1;
            ret += string.substr(bufStart, i - bufStart);
            return ret;
          }
          // falls through
          case "//": {
            i++;
            if (i - 1 > bufStart) {
              ret += string.substr(bufStart, i - bufStart);
            }
            bufStart = i + 1;
            if (i < length) {
              c2 = string[i];
              switch (c2) {
                case "\\":
                  ret += "\\";
                  break;
                case '"':
                  ret += '"';
                  break;
                case "/":
                  ret += "/";
                  break;
                case "b":
                  ret += "\b";
                  break;
                case "f":
                  ret += "\f";
                  break;
                case "n":
                  ret += "\n";
                  break;
                case "r":
                  ret += "\r";
                  break;
                case "t":
                  ret += "	";
                  break;
                case "u":
                  this._error = "parse string/unicord escape not supported";
                  break;
                default:
                  break;
              }
            } else {
              this._error = "parse string/escape error";
            }
          }
          // falls through
          default: {
            break;
          }
        }
      }
      this._error = "parse string/illegal end";
      return null;
    }
    /**
     * JSONのオブジェクトエレメントをパースしてValueオブジェクトを返す
     *
     * @param buffer    JSONエレメントのバッファ
     * @param length    パースする長さ
     * @param begin     パースを開始する位置
     * @param outEndPos パース終了時の位置
     * @return パースから取得したValueオブジェクト
     */
    parseObject(buffer, length, begin, outEndPos) {
      if (this._error) {
        return null;
      }
      if (!buffer) {
        this._error = "buffer is null";
        return null;
      }
      const ret = new JsonMap();
      let key = "";
      let i = begin;
      let c = "";
      const localRetEndPos2 = Array(1);
      let ok = false;
      for (; i < length; i++) {
        FOR_LOOP: for (; i < length; i++) {
          c = buffer[i];
          switch (c) {
            case '"':
              key = this.parseString(buffer, length, i + 1, localRetEndPos2);
              if (this._error) {
                return null;
              }
              i = localRetEndPos2[0];
              ok = true;
              break FOR_LOOP;
            //-- loopから出る
            case "}":
              outEndPos[0] = i + 1;
              return ret;
            // 空
            case ":":
              this._error = "illegal ':' position";
              break;
            case "\n":
              this._lineCount++;
            // falls through
            default:
              break;
          }
        }
        if (!ok) {
          this._error = "key not found";
          return null;
        }
        ok = false;
        FOR_LOOP2: for (; i < length; i++) {
          c = buffer[i];
          switch (c) {
            case ":":
              ok = true;
              i++;
              break FOR_LOOP2;
            case "}":
              this._error = "illegal '}' position";
              break;
            // falls through
            case "\n":
              this._lineCount++;
            // case ' ': case '\t' : case '\r':
            // falls through
            default:
              break;
          }
        }
        if (!ok) {
          this._error = "':' not found";
          return null;
        }
        const value = this.parseValue(buffer, length, i, localRetEndPos2);
        if (this._error) {
          return null;
        }
        i = localRetEndPos2[0];
        ret.put(key, value);
        FOR_LOOP3: for (; i < length; i++) {
          c = buffer[i];
          switch (c) {
            case ",":
              break FOR_LOOP3;
            case "}":
              outEndPos[0] = i + 1;
              return ret;
            // 正常終了
            case "\n":
              this._lineCount++;
            // falls through
            default:
              break;
          }
        }
      }
      this._error = "illegal end of perseObject";
      return null;
    }
    /**
     * 次の「"」までの文字列をパースする。
     * @param buffer    JSONエレメントのバッファ
     * @param length    パースする長さ
     * @param begin     パースを開始する位置
     * @param outEndPos パース終了時の位置
     * @return パースから取得したValueオブジェクト
     */
    parseArray(buffer, length, begin, outEndPos) {
      if (this._error) {
        return null;
      }
      if (!buffer) {
        this._error = "buffer is null";
        return null;
      }
      let ret = new JsonArray();
      let i = begin;
      let c;
      const localRetEndpos2 = new Array(1);
      for (; i < length; i++) {
        const value = this.parseValue(buffer, length, i, localRetEndpos2);
        if (this._error) {
          return null;
        }
        i = localRetEndpos2[0];
        if (value) {
          ret.add(value);
        }
        FOR_LOOP: for (; i < length; i++) {
          c = buffer[i];
          switch (c) {
            case ",":
              break FOR_LOOP;
            case "]":
              outEndPos[0] = i + 1;
              return ret;
            // 終了
            case "\n":
              ++this._lineCount;
            //case ' ': case '\t': case '\r':
            // falls through
            default:
              break;
          }
        }
      }
      ret = void 0;
      this._error = "illegal end of parseObject";
      return null;
    }
    // パースされたルート要素
  };
  var JsonFloat = class extends Value2 {
    /**
     * コンストラクタ
     */
    constructor(v) {
      super();
      this._value = v;
    }
    /**
     * Valueの種類が数値型ならtrue
     */
    isFloat() {
      return true;
    }
    /**
     * 要素を文字列で返す(string型)
     */
    getString(defaultValue, indent) {
      const strbuf = "\0";
      this._value = parseFloat(strbuf);
      this._stringBuffer = strbuf;
      return this._stringBuffer;
    }
    /**
     * 要素を数値型で返す(number)
     */
    toInt(defaultValue = 0) {
      return parseInt(this._value.toString());
    }
    /**
     * 要素を数値型で返す(number)
     */
    toFloat(defaultValue = 0) {
      return this._value;
    }
    equals(value) {
      if ("number" === typeof value) {
        if (Math.round(value)) {
          return false;
        } else {
          return value == this._value;
        }
      }
      return false;
    }
    // JSON要素の値
  };
  var JsonBoolean = class extends Value2 {
    /**
     * Valueの種類が真偽値ならtrue
     */
    isBool() {
      return true;
    }
    /**
     * 要素を真偽値で返す(boolean)
     */
    toBoolean(defaultValue = false) {
      return this._boolValue;
    }
    /**
     * 要素を文字列で返す(string型)
     */
    getString(defaultValue, indent) {
      this._stringBuffer = this._boolValue ? "true" : "false";
      return this._stringBuffer;
    }
    equals(value) {
      if ("boolean" === typeof value) {
        return value == this._boolValue;
      }
      return false;
    }
    /**
     * Valueの値が静的ならtrue, 静的なら解放しない
     */
    isStatic() {
      return true;
    }
    /**
     * 引数付きコンストラクタ
     */
    constructor(v) {
      super();
      this._boolValue = v;
    }
    // JSON要素の値
  };
  var JsonString = class extends Value2 {
    /**
     * 引数付きコンストラクタ
     */
    constructor(s) {
      super();
      this._stringBuffer = s;
    }
    /**
     * Valueの種類が文字列ならtrue
     */
    isString() {
      return true;
    }
    /**
     * 要素を文字列で返す(string型)
     */
    getString(defaultValue, indent) {
      return this._stringBuffer;
    }
    equals(value) {
      if ("string" === typeof value) {
        return this._stringBuffer == value;
      }
      return false;
    }
  };
  var JsonError = class extends JsonString {
    /**
     * Valueの値が静的ならtrue、静的なら解放しない
     */
    isStatic() {
      return this._isStatic;
    }
    /**
     * エラー情報をセットする
     */
    setErrorNotForClientCall(s) {
      this._stringBuffer = s;
      return this;
    }
    /**
     * 引数付きコンストラクタ
     */
    constructor(s, isStatic) {
      if ("string" === typeof s) {
        super(s);
      } else {
        super(s);
      }
      this._isStatic = isStatic;
    }
    /**
     * Valueの種類がエラー値ならtrue
     */
    isError() {
      return true;
    }
    // 静的なValueかどうか
  };
  var JsonNullvalue = class extends Value2 {
    /**
     * Valueの種類がNULL値ならtrue
     */
    isNull() {
      return true;
    }
    /**
     * 要素を文字列で返す(string型)
     */
    getString(defaultValue, indent) {
      return this._stringBuffer;
    }
    /**
     * Valueの値が静的ならtrue, 静的なら解放しない
     */
    isStatic() {
      return true;
    }
    /**
     * Valueにエラー値をセットする
     */
    setErrorNotForClientCall(s) {
      this._stringBuffer = s;
      return JsonError.nullValue;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._stringBuffer = "NullValue";
    }
  };
  var JsonArray = class extends Value2 {
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._array = new Array();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      for (let i = 0; i < this._array.length; i++) {
        let v = this._array[i];
        if (v && !v.isStatic()) {
          v = void 0;
          v = null;
        }
      }
    }
    /**
     * Valueの種類が配列ならtrue
     */
    isArray() {
      return true;
    }
    /**
     * 添字演算子[index]
     */
    getValueByIndex(index) {
      if (index < 0 || this._array.length <= index) {
        return Value2.errorValue.setErrorNotForClientCall(
          CSM_JSON_ERROR_INDEX_OF_BOUNDS
        );
      }
      const v = this._array[index];
      if (v == null) {
        return Value2.nullValue;
      }
      return v;
    }
    /**
     * 添字演算子[string]
     */
    getValueByString(s) {
      return Value2.errorValue.setErrorNotForClientCall(
        CSM_JSON_ERROR_TYPE_MISMATCH
      );
    }
    /**
     * 要素を文字列で返す(string型)
     */
    getString(defaultValue, indent) {
      const stringBuffer = indent + "[\n";
      for (let i = 0; i < this._array.length; i++) {
        const v = this._array[i];
        this._stringBuffer += indent + "" + v.getString(indent + " ") + "\n";
      }
      this._stringBuffer = stringBuffer + indent + "]\n";
      return this._stringBuffer;
    }
    /**
     * 配列要素を追加する
     * @param v 追加する要素
     */
    add(v) {
      this._array.push(v);
    }
    /**
     * 要素をコンテナで返す(Array<Value>)
     */
    getVector(defaultValue = null) {
      return this._array;
    }
    /**
     * 要素の数を返す
     */
    getSize() {
      return this._array.length;
    }
    // JSON要素の値
  };
  var JsonMap = class extends Value2 {
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._map = /* @__PURE__ */ new Map();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this._map.clear();
    }
    /**
     * Valueの値がMap型ならtrue
     */
    isMap() {
      return true;
    }
    /**
     * 添字演算子[string]
     */
    getValueByString(s) {
      const ret = this._map.get(s);
      if (ret != void 0) {
        return ret;
      }
      return Value2.nullValue;
    }
    /**
     * 添字演算子[index]
     */
    getValueByIndex(index) {
      return Value2.errorValue.setErrorNotForClientCall(
        CSM_JSON_ERROR_TYPE_MISMATCH
      );
    }
    /**
     * 要素を文字列で返す(string型)
     */
    getString(defaultValue, indent) {
      this._stringBuffer = indent + "{\n";
      for (const element of this._map) {
        const key = element[0];
        const v = element[1];
        this._stringBuffer += indent + " " + key + " : " + v.getString(indent + "   ") + " \n";
      }
      this._stringBuffer += indent + "}\n";
      return this._stringBuffer;
    }
    /**
     * 要素をMap型で返す
     */
    getMap(defaultValue) {
      return this._map;
    }
    /**
     * Mapに要素を追加する
     */
    put(key, v) {
      this._map.set(key, v);
    }
    /**
     * Mapからキーのリストを取得する
     */
    getKeys() {
      if (!this._keys) {
        this._keys = [...this._map.keys()];
      }
      return this._keys;
    }
    /**
     * Mapの要素数を取得する
     */
    getSize() {
      return this._keys.length;
    }
    // JSON要素の値
  };
  var Live2DCubismFramework9;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismJson = CubismJson;
    Live2DCubismFramework51.JsonArray = JsonArray;
    Live2DCubismFramework51.JsonBoolean = JsonBoolean;
    Live2DCubismFramework51.JsonError = JsonError;
    Live2DCubismFramework51.JsonFloat = JsonFloat;
    Live2DCubismFramework51.JsonMap = JsonMap;
    Live2DCubismFramework51.JsonNullvalue = JsonNullvalue;
    Live2DCubismFramework51.JsonString = JsonString;
    Live2DCubismFramework51.Value = Value2;
  })(Live2DCubismFramework9 || (Live2DCubismFramework9 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/live2dcubismframework.ts
  function strtod(s, endPtr) {
    let index = 0;
    for (let i = 1; ; i++) {
      const testC = s.slice(i - 1, i);
      if (testC == "e" || testC == "-" || testC == "E") {
        continue;
      }
      const test = s.substring(0, i);
      const number = Number(test);
      if (isNaN(number)) {
        break;
      }
      index = i;
    }
    let d = parseFloat(s);
    if (isNaN(d)) {
      d = NaN;
    }
    endPtr[0] = s.slice(index);
    return d;
  }
  var s_isStarted = false;
  var s_isInitialized = false;
  var s_option = null;
  var s_cubismIdManager = null;
  var Constant = Object.freeze({
    vertexOffset: 0,
    // メッシュ頂点のオフセット値
    vertexStep: 2
    // メッシュ頂点のステップ値
  });
  function csmDelete(address) {
    if (!address) {
      return;
    }
    address = void 0;
  }
  var CubismFramework = class {
    /**
     * Cubism FrameworkのAPIを使用可能にする。
     *  APIを実行する前に必ずこの関数を実行すること。
     *  一度準備が完了して以降は、再び実行しても内部処理がスキップされます。
     *
     * @param    option      Optionクラスのインスタンス
     *
     * @return   準備処理が完了したらtrueが返ります。
     */
    static startUp(option = null) {
      if (s_isStarted) {
        CubismLogInfo("CubismFramework.startUp() is already done.");
        return s_isStarted;
      }
      s_option = option;
      if (s_option != null) {
        Live2DCubismCore.Logging.csmSetLogFunction(s_option.logFunction);
      }
      s_isStarted = true;
      if (s_isStarted) {
        const version = Live2DCubismCore.Version.csmGetVersion();
        const major = (version & 4278190080) >> 24;
        const minor = (version & 16711680) >> 16;
        const patch = version & 65535;
        const versionNumber = version;
        CubismLogInfo(
          `Live2D Cubism Core version: {0}.{1}.{2} ({3})`,
          ("00" + major).slice(-2),
          ("00" + minor).slice(-2),
          ("0000" + patch).slice(-4),
          versionNumber
        );
      }
      CubismLogInfo("CubismFramework.startUp() is complete.");
      return s_isStarted;
    }
    /**
     * StartUp()で初期化したCubismFrameworkの各パラメータをクリアします。
     * Dispose()したCubismFrameworkを再利用する際に利用してください。
     */
    static cleanUp() {
      s_isStarted = false;
      s_isInitialized = false;
      s_option = null;
      s_cubismIdManager = null;
    }
    /**
     * Cubism Framework内のリソースを初期化してモデルを表示可能な状態にします。<br>
     *     再度Initialize()するには先にDispose()を実行する必要があります。
     *
     * @param memorySize 初期化時メモリ量 [byte(s)]
     *    複数モデル表示時などにモデルが更新されない際に使用してください。
     *    指定する際は必ず1024*1024*16 byte(16MB)以上の値を指定してください。
     *    それ以外はすべて1024*1024*16 byteに丸めます。
     */
    static initialize(memorySize = 0) {
      CSM_ASSERT(s_isStarted);
      if (!s_isStarted) {
        CubismLogWarning("CubismFramework is not started.");
        return;
      }
      if (s_isInitialized) {
        CubismLogWarning(
          "CubismFramework.initialize() skipped, already initialized."
        );
        return;
      }
      Value2.staticInitializeNotForClientCall();
      s_cubismIdManager = new CubismIdManager();
      Live2DCubismCore.Memory.initializeAmountOfMemory(memorySize);
      s_isInitialized = true;
      CubismLogInfo("CubismFramework.initialize() is complete.");
    }
    /**
     * Cubism Framework内の全てのリソースを解放します。
     *      ただし、外部で確保されたリソースについては解放しません。
     *      外部で適切に破棄する必要があります。
     */
    static dispose() {
      CSM_ASSERT(s_isStarted);
      if (!s_isStarted) {
        CubismLogWarning("CubismFramework is not started.");
        return;
      }
      if (!s_isInitialized) {
        CubismLogWarning("CubismFramework.dispose() skipped, not initialized.");
        return;
      }
      Value2.staticReleaseNotForClientCall();
      s_cubismIdManager.release();
      s_cubismIdManager = null;
      CubismRenderer.staticRelease();
      s_isInitialized = false;
      CubismLogInfo("CubismFramework.dispose() is complete.");
    }
    /**
     * Cubism FrameworkのAPIを使用する準備が完了したかどうか
     * @return APIを使用する準備が完了していればtrueが返ります。
     */
    static isStarted() {
      return s_isStarted;
    }
    /**
     * Cubism Frameworkのリソース初期化がすでに行われているかどうか
     * @return リソース確保が完了していればtrueが返ります
     */
    static isInitialized() {
      return s_isInitialized;
    }
    /**
     * Core APIにバインドしたログ関数を実行する
     *
     * @praram message ログメッセージ
     */
    static coreLogFunction(message) {
      if (!Live2DCubismCore.Logging.csmGetLogFunction()) {
        return;
      }
      Live2DCubismCore.Logging.csmGetLogFunction()(message);
    }
    /**
     * 現在のログ出力レベル設定の値を返す。
     *
     * @return  現在のログ出力レベル設定の値
     */
    static getLoggingLevel() {
      if (s_option != null) {
        return s_option.loggingLevel;
      }
      return 5 /* LogLevel_Off */;
    }
    /**
     * IDマネージャのインスタンスを取得する
     * @return CubismManagerクラスのインスタンス
     */
    static getIdManager() {
      return s_cubismIdManager;
    }
    /**
     * 静的クラスとして使用する
     * インスタンス化させない
     */
    constructor() {
    }
  };
  var Option = class {
    // ログ出力レベルの設定
  };
  var Live2DCubismFramework10;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.Constant = Constant;
    Live2DCubismFramework51.csmDelete = csmDelete;
    Live2DCubismFramework51.CubismFramework = CubismFramework;
  })(Live2DCubismFramework10 || (Live2DCubismFramework10 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lapppal.ts
  var LAppPal = class {
    /**
     * ファイルをバイトデータとして読みこむ
     *
     * @param filePath 読み込み対象ファイルのパス
     * @return
     * {
     *      buffer,   読み込んだバイトデータ
     *      size        ファイルサイズ
     * }
     */
    static loadFileAsBytes(filePath, callback) {
      fetch(filePath).then((response) => response.arrayBuffer()).then((arrayBuffer) => callback(arrayBuffer, arrayBuffer.byteLength));
    }
    /**
     * デルタ時間（前回フレームとの差分）を取得する
     * @return デルタ時間[ms]
     */
    static getDeltaTime() {
      return this.deltaTime;
    }
    static updateTime() {
      this.currentFrame = Date.now();
      this.deltaTime = (this.currentFrame - this.lastFrame) / 1e3;
      this.lastFrame = this.currentFrame;
    }
    /**
     * メッセージを出力する
     * @param message 文字列
     */
    static printMessage(message) {
      console.log(message);
    }
  };
  LAppPal.lastUpdate = Date.now();
  LAppPal.currentFrame = 0;
  LAppPal.lastFrame = 0;
  LAppPal.deltaTime = 0;

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappdefine.ts
  var CanvasSize = "auto";
  var ViewScale = 1;
  var ViewMaxScale = 2;
  var ViewMinScale = 0.8;
  var ViewLogicalLeft = -1;
  var ViewLogicalRight = 1;
  var ViewLogicalMaxLeft = -2;
  var ViewLogicalMaxRight = 2;
  var ViewLogicalMaxBottom = -2;
  var ViewLogicalMaxTop = 2;
  var ResourcesPath = "/resources/models/AmadeusKurisu5.0/";
  var ShaderPath = "/resources/live2d/shaders/";
  var BackImageName = "";
  var GearImageName = "";
  var ModelDir = [
    "reama5.0"
  ];
  var ModelDirSize = ModelDir.length;
  var ModelJsonFileName = "Live2D\u7D05\u8389\u6816forSDK5.0.model3.json";
  var MotionGroupIdle = "Idle";
  var MotionGroupTapBody = "TapBody";
  var HitAreaNameHead = "Head";
  var HitAreaNameBody = "Body";
  var PriorityNone = 0;
  var PriorityIdle = 1;
  var PriorityNormal = 2;
  var PriorityForce = 3;
  var MOCConsistencyValidationEnable = false;
  var MotionConsistencyValidationEnable = false;
  var DebugLogEnable = false;
  var DebugTouchLogEnable = false;
  var CubismLoggingLevel = 0 /* LogLevel_Verbose */;

  var s_arrayBufferCache = /* @__PURE__ */ new Map();
  var s_textCache = /* @__PURE__ */ new Map();
  var s_imageCache = /* @__PURE__ */ new Map();
  function cachedFetchArrayBuffer(url) {
    let p = s_arrayBufferCache.get(url);
    if (!p) {
      p = fetch(url).then((res) => {
        if (!res.ok) {
          throw new Error(`Failed to fetch ${url}: ${res.status}`);
        }
        return res.arrayBuffer();
      }).catch((err) => {
        s_arrayBufferCache.delete(url);
        throw err;
      });
      s_arrayBufferCache.set(url, p);
    }
    return p;
  }
  function cachedFetchText(url) {
    let p = s_textCache.get(url);
    if (!p) {
      p = fetch(url).then((res) => {
        if (!res.ok) {
          throw new Error(`Failed to fetch ${url}: ${res.status}`);
        }
        return res.text();
      }).catch((err) => {
        s_textCache.delete(url);
        throw err;
      });
      s_textCache.set(url, p);
    }
    return p;
  }
  function cachedLoadImage(url) {
    let p = s_imageCache.get(url);
    if (!p) {
      p = new Promise((resolve, reject) => {
        const img = new Image();
        img.decoding = "async";
        img.addEventListener(
          "load",
          () => {
            if (typeof img.decode === "function") {
              img.decode().then(() => resolve(img), () => resolve(img));
            } else {
              resolve(img);
            }
          },
          { once: true, passive: true }
        );
        img.addEventListener(
          "error",
          (err) => {
            s_imageCache.delete(url);
            reject(err);
          },
          { once: true, passive: true }
        );
        img.src = url;
      });
      s_imageCache.set(url, p);
    }
    return p;
  }

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappglmanager.ts
  var LAppGlManager = class {
    constructor() {
      this._gl = null;
      this._gl = null;
    }
    initialize(canvas) {
      const contextAttributes = {
        alpha: true,
        antialias: true,
        premultipliedAlpha: true,
        preserveDrawingBuffer: false
      };
      this._gl = canvas.getContext("webgl", contextAttributes);
      if (!this._gl) {
        this._gl = canvas.getContext("webgl2", contextAttributes);
      }
      if (!this._gl) {
        console.error("Cannot initialize WebGL context.");
        this._gl = null;
        return false;
      }
      return true;
    }
    /**
     * 解放する。
     */
    release() {
    }
    getGl() {
      return this._gl;
    }
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/utils/cubismarrayutils.ts
  function updateSize(curArray, newSize, value = null, callPlacementNew = null) {
    const curSize = curArray.length;
    if (curSize < newSize) {
      if (callPlacementNew) {
        for (let i = curArray.length; i < newSize; i++) {
          if (typeof value == "function") {
            curArray[i] = JSON.parse(JSON.stringify(new value()));
          } else {
            curArray[i] = value;
          }
        }
      } else {
        for (let i = curArray.length; i < newSize; i++) {
          curArray[i] = value;
        }
      }
    } else {
      curArray.length = newSize;
    }
  }

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismrendertarget_webgl.ts
  var CubismRenderTarget_WebGL = class {
    /**
     * WebGL2RenderingContext.blitFramebuffer() でバッファのコピーを行う。
     *
     * @param src コピー元のオフスクリーンサーフェス
     * @param dst コピー先のオフスクリーンサーフェス
     */
    static copyBuffer(gl, src, dst) {
      if (src == null || dst == null) {
        return;
      }
      if (!(gl instanceof WebGL2RenderingContext)) {
        throw new Error("WebGL2RenderingContext is required for buffer copy.");
      }
      const previousFramebuffer = gl.getParameter(
        gl.FRAMEBUFFER_BINDING
      );
      gl.bindFramebuffer(gl.READ_FRAMEBUFFER, src.getRenderTexture());
      gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, dst.getRenderTexture());
      gl.blitFramebuffer(
        0,
        0,
        src.getBufferWidth(),
        src.getBufferHeight(),
        0,
        0,
        dst.getBufferWidth(),
        dst.getBufferHeight(),
        gl.COLOR_BUFFER_BIT,
        gl.NEAREST
      );
      gl.bindFramebuffer(gl.FRAMEBUFFER, previousFramebuffer);
    }
    /**
     * 描画を開始する。
     *
     * @param restoreFbo EndDraw時に復元するFBOを指定する。nullを指定すると、beginDraw時に現在のFBOを記憶しておく。
     */
    beginDraw(restoreFbo = null) {
      if (this._renderTexture == null) {
        console.error("_renderTexture is null");
        return;
      }
      if (restoreFbo == null) {
        this._oldFbo = this._gl.getParameter(this._gl.FRAMEBUFFER_BINDING);
      } else {
        this._oldFbo = restoreFbo;
      }
      this._gl.bindFramebuffer(this._gl.FRAMEBUFFER, this._renderTexture);
    }
    /**
     * 描画を終了し、バックバッファのサーフェイスを復元する。
     */
    endDraw() {
      this._gl.bindFramebuffer(this._gl.FRAMEBUFFER, this._oldFbo);
    }
    /**
     * バインドされているカラーバッファのクリアを行う。
     *
     * @param r 赤の成分 (0.0 - 1.0)
     * @param g 緑の成分 (0.0 - 1.0)
     * @param b 青の成分 (0.0 - 1.0)
     * @param a アルファの成分 (0.0 - 1.0)
     */
    clear(r, g, b, a) {
      this._gl.clearColor(r, g, b, a);
      this._gl.clear(this._gl.COLOR_BUFFER_BIT);
    }
    /**
     * オフスクリーンサーフェスを作成する。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     *          NOTE: Cubism 5.3以降のモデルが使用される場合はWebGL2RenderingContextを使用すること。
     * @param displayBufferWidth オフスクリーンサーフェスの幅
     * @param displayBufferHeight オフスクリーンサーフェスの高さ
     * @param previousFramebuffer 前のフレームバッファ
     *
     * @return 成功した場合はtrue、失敗した場合はfalse
     */
    createRenderTarget(gl, displayBufferWidth, displayBufferHeight, previousFramebuffer) {
      this.destroyRenderTarget();
      this._colorBuffer = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, this._colorBuffer);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        displayBufferWidth,
        displayBufferHeight,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        null
      );
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.bindTexture(gl.TEXTURE_2D, null);
      const ret = gl.createFramebuffer();
      if (ret == null) {
        CubismLogError("Failed to create framebuffer");
        return false;
      }
      gl.bindFramebuffer(gl.FRAMEBUFFER, ret);
      gl.framebufferTexture2D(
        gl.FRAMEBUFFER,
        gl.COLOR_ATTACHMENT0,
        gl.TEXTURE_2D,
        this._colorBuffer,
        0
      );
      const status = gl.checkFramebufferStatus(gl.FRAMEBUFFER);
      if (status !== gl.FRAMEBUFFER_COMPLETE) {
        CubismLogError("Framebuffer is not complete");
        gl.bindFramebuffer(gl.FRAMEBUFFER, previousFramebuffer);
        gl.deleteFramebuffer(ret);
        this.destroyRenderTarget();
        return false;
      }
      this._renderTexture = ret;
      this._bufferWidth = displayBufferWidth;
      this._bufferHeight = displayBufferHeight;
      this._gl = gl;
      return true;
    }
    /**
     * レンダーターゲットを破棄する。
     */
    destroyRenderTarget() {
      if (this._colorBuffer) {
        this._gl.bindTexture(this._gl.TEXTURE_2D, null);
        this._gl.deleteTexture(this._colorBuffer);
        this._colorBuffer = null;
      }
      if (this._renderTexture) {
        this._gl.bindFramebuffer(this._gl.FRAMEBUFFER, null);
        this._gl.deleteFramebuffer(this._renderTexture);
        this._renderTexture = null;
      }
    }
    /**
     * WebGLのコンテキストを取得する。
     *
     * @return WebGLRenderingContextまたはWebGL2RenderingContext
     */
    getGL() {
      return this._gl;
    }
    /**
     * レンダーテクスチャを取得する。
     *
     * @return WebGLFramebuffer
     */
    getRenderTexture() {
      return this._renderTexture;
    }
    /**
     * カラーバッファを取得する。
     *
     * @return WebGLTexture
     */
    getColorBuffer() {
      return this._colorBuffer;
    }
    /**
     * カラーバッファの幅を取得する。
     *
     * @return カラーバッファの幅
     */
    getBufferWidth() {
      return this._bufferWidth;
    }
    /**
     * カラーバッファの高さを取得する。
     *
     * @return カラーバッファの高さ
     */
    getBufferHeight() {
      return this._bufferHeight;
    }
    /**
     * オフスクリーンサーフェスが有効かどうかを確認する。
     *
     * @return 有効な場合はtrue、無効な場合はfalse
     */
    isValid() {
      return this._renderTexture != null;
    }
    /**
     * 以前のフレームバッファを取得する。
     *
     * @return 以前のフレームバッファ
     */
    getOldFBO() {
      return this._oldFbo;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._gl = null;
      this._colorBuffer = null;
      this._renderTexture = null;
      this._bufferWidth = 0;
      this._bufferHeight = 0;
      this._oldFbo = null;
    }
    // 以前のフレームバッファ
  };
  var Live2DCubismFramework11;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismOffscreenSurface_WebGL = CubismRenderTarget_WebGL;
  })(Live2DCubismFramework11 || (Live2DCubismFramework11 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismoffscreenmanager.ts
  var CubismRenderTargetContainer = class {
    /**
     * Constructor
     *
     * @param colorBuffer カラーバッファ
     * @param renderTexture レンダーテクスチャ
     * @param inUse 使用中かどうか
     */
    constructor(colorBuffer = null, renderTexture = null, inUse = false) {
      this.colorBuffer = colorBuffer;
      this.renderTexture = renderTexture;
      this.inUse = inUse;
    }
    clear() {
      this.colorBuffer = null;
      this.renderTexture = null;
      this.inUse = false;
    }
    /**
     * カラーバッファを取得
     *
     * @returns カラーバッファ
     */
    getColorBuffer() {
      return this.colorBuffer;
    }
    /**
     * レンダーテクスチャを取得
     *
     * @returns レンダーテクスチャ
     */
    getRenderTexture() {
      return this.renderTexture;
    }
    // Whether this container's render target is currently in use
  };
  var CubismWebGLContextManager = class {
    constructor(gl) {
      this.gl = gl;
      this.offscreenRenderTargetContainers = new Array();
      this.previousActiveRenderTextureMaxCount = 0;
      this.currentActiveRenderTextureCount = 0;
      this.hasResetThisFrame = false;
      this.width = 0;
      this.height = 0;
    }
    release() {
      if (this.offscreenRenderTargetContainers != null) {
        for (let index = 0; index < this.offscreenRenderTargetContainers.length; ++index) {
          const container = this.offscreenRenderTargetContainers[index];
          this.gl.deleteTexture(container.colorBuffer);
          this.gl.deleteFramebuffer(container.renderTexture);
        }
        this.offscreenRenderTargetContainers.length = 0;
        this.offscreenRenderTargetContainers = null;
      }
    }
    // 高さ
  };
  var CubismWebGLOffscreenManager = class _CubismWebGLOffscreenManager {
    /**
     * コンストラクタ
     */
    constructor() {
      this._contextManagers = /* @__PURE__ */ new Map();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      if (this._contextManagers != null) {
        for (const manager of this._contextManagers.values()) {
          manager.release();
        }
        this._contextManagers.clear();
        this._contextManagers = null;
      }
      _CubismWebGLOffscreenManager._instance = null;
    }
    /**
     * インスタンスの取得
     *
     * @return インスタンス
     */
    static getInstance() {
      if (this._instance == null) {
        this._instance = new _CubismWebGLOffscreenManager();
      }
      return this._instance;
    }
    /**
     * WebGLContextに対応するマネージャーを取得または作成
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @return WebGLContextManager
     */
    getContextManager(gl) {
      if (!this._contextManagers.has(gl)) {
        this._contextManagers.set(gl, new CubismWebGLContextManager(gl));
      }
      return this._contextManagers.get(gl);
    }
    /**
     * 指定されたWebGLContextのマネージャーを削除
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    removeContext(gl) {
      if (this._contextManagers.has(gl)) {
        const manager = this._contextManagers.get(gl);
        manager.release();
        this._contextManagers.delete(gl);
      }
    }
    /**
     * 初期化処理
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @param width 幅
     * @param height 高さ
     */
    initialize(gl, width, height) {
      const contextManager = this.getContextManager(gl);
      if (contextManager.offscreenRenderTargetContainers != null) {
        for (let index = 0; index < contextManager.offscreenRenderTargetContainers.length; ++index) {
          const container = contextManager.offscreenRenderTargetContainers[index];
          contextManager.gl.deleteTexture(container.colorBuffer);
          contextManager.gl.deleteFramebuffer(container.renderTexture);
          container.clear();
        }
        contextManager.offscreenRenderTargetContainers.length = 0;
      } else {
        contextManager.offscreenRenderTargetContainers = new Array();
      }
      contextManager.width = width;
      contextManager.height = height;
      contextManager.previousActiveRenderTextureMaxCount = 0;
      contextManager.currentActiveRenderTextureCount = 0;
      contextManager.hasResetThisFrame = false;
    }
    /**
     * モデルを描画する前に呼び出すフレーム開始時の処理を行う
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    beginFrameProcess(gl) {
      const contextManager = this.getContextManager(gl);
      if (contextManager.hasResetThisFrame) {
        return;
      }
      contextManager.previousActiveRenderTextureMaxCount = 0;
      contextManager.hasResetThisFrame = true;
    }
    /**
     * モデルの描画が終わった後に呼び出すフレーム終了時の処理
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    endFrameProcess(gl) {
      const contextManager = this.getContextManager(gl);
      contextManager.hasResetThisFrame = false;
    }
    /**
     * コンテナサイズの取得
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    getContainerSize(gl) {
      const contextManager = this.getContextManager(gl);
      if (contextManager.offscreenRenderTargetContainers == null) {
        return 0;
      }
      return contextManager.offscreenRenderTargetContainers.length;
    }
    /**
     * 使用可能なリソースコンテナの取得
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @param width 幅
     * @param height 高さ
     * @param previousFramebuffer 前のフレームバッファ
     * @return 使用可能なリソースコンテナ
     */
    getOffscreenRenderTargetContainers(gl, width, height, previousFramebuffer) {
      const contextManager = this.getContextManager(gl);
      if (contextManager.width != width || contextManager.height != height || contextManager.offscreenRenderTargetContainers == null) {
        this.initialize(gl, width, height);
      }
      this.updateRenderTargetContainerCount(gl);
      const container = this.getUnusedOffscreenRenderTargetContainer(gl);
      if (container != null) {
        return container;
      }
      const offscreenRenderTextureContainer = this.createOffscreenRenderTargetContainer(
        gl,
        width,
        height,
        previousFramebuffer
      );
      return offscreenRenderTextureContainer;
    }
    /**
     * リソースコンテナの使用状態を取得
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @param renderTexture WebGLFramebuffer
     * @return 使用中はtrue、未使用の場合はfalse
     */
    getUsingRenderTextureState(gl, renderTexture) {
      const contextManager = this.getContextManager(gl);
      for (let index = 0; index < contextManager.offscreenRenderTargetContainers.length; ++index) {
        if (contextManager.offscreenRenderTargetContainers[index].renderTexture == renderTexture) {
          return contextManager.offscreenRenderTargetContainers[index].inUse;
        }
      }
      return true;
    }
    /**
     * リソースコンテナの使用を開始する。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @param renderTexture WebGLFramebuffer
     */
    startUsingRenderTexture(gl, renderTexture) {
      const contextManager = this.getContextManager(gl);
      for (let index = 0; index < contextManager.offscreenRenderTargetContainers.length; ++index) {
        if (contextManager.offscreenRenderTargetContainers[index].renderTexture != renderTexture) {
          continue;
        }
        contextManager.offscreenRenderTargetContainers[index].inUse = true;
        this.updateRenderTargetContainerCount(gl);
        break;
      }
    }
    /**
     * リソースコンテナの使用を終了する。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @param renderTexture WebGLFramebuffer
     */
    stopUsingRenderTexture(gl, renderTexture) {
      const contextManager = this.getContextManager(gl);
      for (let index = 0; index < contextManager.offscreenRenderTargetContainers.length; ++index) {
        if (contextManager.offscreenRenderTargetContainers[index].renderTexture != renderTexture) {
          continue;
        }
        contextManager.offscreenRenderTargetContainers[index].inUse = false;
        contextManager.currentActiveRenderTextureCount--;
        if (contextManager.currentActiveRenderTextureCount < 0) {
          contextManager.currentActiveRenderTextureCount = 0;
        }
        break;
      }
    }
    /**
     * リソースコンテナの使用を全て終了する。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    stopUsingAllRenderTextures(gl) {
      const contextManager = this.getContextManager(gl);
      for (let index = 0; index < contextManager.offscreenRenderTargetContainers.length; ++index) {
        contextManager.offscreenRenderTargetContainers[index].inUse = false;
      }
      contextManager.currentActiveRenderTextureCount = 0;
    }
    /**
     * 使用されていないリソースコンテナを解放する。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    releaseStaleRenderTextures(gl) {
      const contextManager = this.getContextManager(gl);
      const listSize = contextManager.offscreenRenderTargetContainers.length;
      if (contextManager.hasResetThisFrame || listSize === 0) {
        return;
      }
      let findPos = 0;
      let resize = contextManager.previousActiveRenderTextureMaxCount;
      for (let i = listSize; contextManager.previousActiveRenderTextureMaxCount < i; --i) {
        const index = i - 1;
        if (contextManager.offscreenRenderTargetContainers[index].inUse) {
          let isFind = false;
          for (; findPos < contextManager.previousActiveRenderTextureMaxCount; ++findPos) {
            if (!contextManager.offscreenRenderTargetContainers[findPos].inUse) {
              const tempContainer = contextManager.offscreenRenderTargetContainers[findPos];
              contextManager.offscreenRenderTargetContainers[findPos] = contextManager.offscreenRenderTargetContainers[index];
              contextManager.offscreenRenderTargetContainers[findPos].inUse = true;
              contextManager.offscreenRenderTargetContainers[index] = tempContainer;
              contextManager.offscreenRenderTargetContainers[index].inUse = false;
              isFind = true;
              break;
            }
          }
          if (!isFind) {
            resize = i;
            break;
          }
        }
        const container = contextManager.offscreenRenderTargetContainers[index];
        contextManager.gl.bindTexture(contextManager.gl.TEXTURE_2D, null);
        contextManager.gl.deleteTexture(container.colorBuffer);
        contextManager.gl.bindFramebuffer(contextManager.gl.FRAMEBUFFER, null);
        contextManager.gl.deleteFramebuffer(container.renderTexture);
        container.clear();
      }
      updateSize(contextManager.offscreenRenderTargetContainers, resize);
    }
    /**
     * 直前のアクティブなレンダーターゲットの最大数を取得
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @returns 直前のアクティブなレンダーターゲットの最大数
     */
    getPreviousActiveRenderTextureCount(gl) {
      const contextManager = this.getContextManager(gl);
      return contextManager.previousActiveRenderTextureMaxCount;
    }
    /**
     * 現在のアクティブなレンダーターゲットの数を取得
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @returns 現在のアクティブなレンダーターゲットの数
     */
    getCurrentActiveRenderTextureCount(gl) {
      const contextManager = this.getContextManager(gl);
      return contextManager.currentActiveRenderTextureCount;
    }
    /**
     * 現在のアクティブなレンダーターゲットの数を更新
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     */
    updateRenderTargetContainerCount(gl) {
      const contextManager = this.getContextManager(gl);
      ++contextManager.currentActiveRenderTextureCount;
      contextManager.previousActiveRenderTextureMaxCount = contextManager.currentActiveRenderTextureCount > contextManager.previousActiveRenderTextureMaxCount ? contextManager.currentActiveRenderTextureCount : contextManager.previousActiveRenderTextureMaxCount;
    }
    /**
     * 使用されていないリソースコンテナの取得
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @return 使用されていないリソースコンテナ
     */
    getUnusedOffscreenRenderTargetContainer(gl) {
      const contextManager = this.getContextManager(gl);
      for (let index = 0; index < contextManager.offscreenRenderTargetContainers.length; ++index) {
        const container = contextManager.offscreenRenderTargetContainers[index];
        if (container.inUse == false) {
          container.inUse = true;
          return container;
        }
      }
      return null;
    }
    /**
     * 新たにリソースコンテナを作成する。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     * @param width 幅
     * @param height 高さ
     * @param previousFramebuffer 前のフレームバッファ
     * @return 作成されたリソースコンテナ
     */
    createOffscreenRenderTargetContainer(gl, width, height, previousFramebuffer) {
      const renderTarget = new CubismRenderTarget_WebGL();
      if (!renderTarget.createRenderTarget(gl, width, height, previousFramebuffer)) {
        CubismLogError("Failed to create offscreen render texture.");
        return null;
      }
      const offscreenRenderTextureContainer = new CubismRenderTargetContainer(
        renderTarget.getColorBuffer(),
        renderTarget.getRenderTexture(),
        true
      );
      const contextManager = this.getContextManager(gl);
      contextManager.offscreenRenderTargetContainers.push(
        offscreenRenderTextureContainer
      );
      return offscreenRenderTextureContainer;
    }
    // WebGLContextごとのマネージャー
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/cubismdefaultparameterid.ts
  var CubismDefaultParameterId = Object.freeze({
    // パーツID
    HitAreaPrefix: "HitArea",
    HitAreaHead: "Head",
    HitAreaBody: "Body",
    PartsIdCore: "Parts01Core",
    PartsArmPrefix: "Parts01Arm_",
    PartsArmLPrefix: "Parts01ArmL_",
    PartsArmRPrefix: "Parts01ArmR_",
    // パラメータID
    ParamAngleX: "ParamAngleX",
    ParamAngleY: "ParamAngleY",
    ParamAngleZ: "ParamAngleZ",
    ParamEyeLOpen: "ParamEyeLOpen",
    ParamEyeLSmile: "ParamEyeLSmile",
    ParamEyeROpen: "ParamEyeROpen",
    ParamEyeRSmile: "ParamEyeRSmile",
    ParamEyeBallX: "ParamEyeBallX",
    ParamEyeBallY: "ParamEyeBallY",
    ParamEyeBallForm: "ParamEyeBallForm",
    ParamBrowLY: "ParamBrowLY",
    ParamBrowRY: "ParamBrowRY",
    ParamBrowLX: "ParamBrowLX",
    ParamBrowRX: "ParamBrowRX",
    ParamBrowLAngle: "ParamBrowLAngle",
    ParamBrowRAngle: "ParamBrowRAngle",
    ParamBrowLForm: "ParamBrowLForm",
    ParamBrowRForm: "ParamBrowRForm",
    ParamMouthForm: "ParamMouthForm",
    ParamMouthOpenY: "ParamMouthOpenY",
    ParamCheek: "ParamCheek",
    ParamBodyAngleX: "ParamBodyAngleX",
    ParamBodyAngleY: "ParamBodyAngleY",
    ParamBodyAngleZ: "ParamBodyAngleZ",
    ParamBreath: "ParamBreath",
    ParamArmLA: "ParamArmLA",
    ParamArmRA: "ParamArmRA",
    ParamArmLB: "ParamArmLB",
    ParamArmRB: "ParamArmRB",
    ParamHandL: "ParamHandL",
    ParamHandR: "ParamHandR",
    ParamHairFront: "ParamHairFront",
    ParamHairSide: "ParamHairSide",
    ParamHairBack: "ParamHairBack",
    ParamHairFluffy: "ParamHairFluffy",
    ParamShoulderY: "ParamShoulderY",
    ParamBustX: "ParamBustX",
    ParamBustY: "ParamBustY",
    ParamBaseX: "ParamBaseX",
    ParamBaseY: "ParamBaseY",
    ParamNONE: "NONE:"
  });
  var Live2DCubismFramework12;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.HitAreaBody = CubismDefaultParameterId.HitAreaBody;
    Live2DCubismFramework51.HitAreaHead = CubismDefaultParameterId.HitAreaHead;
    Live2DCubismFramework51.HitAreaPrefix = CubismDefaultParameterId.HitAreaPrefix;
    Live2DCubismFramework51.ParamAngleX = CubismDefaultParameterId.ParamAngleX;
    Live2DCubismFramework51.ParamAngleY = CubismDefaultParameterId.ParamAngleY;
    Live2DCubismFramework51.ParamAngleZ = CubismDefaultParameterId.ParamAngleZ;
    Live2DCubismFramework51.ParamArmLA = CubismDefaultParameterId.ParamArmLA;
    Live2DCubismFramework51.ParamArmLB = CubismDefaultParameterId.ParamArmLB;
    Live2DCubismFramework51.ParamArmRA = CubismDefaultParameterId.ParamArmRA;
    Live2DCubismFramework51.ParamArmRB = CubismDefaultParameterId.ParamArmRB;
    Live2DCubismFramework51.ParamBaseX = CubismDefaultParameterId.ParamBaseX;
    Live2DCubismFramework51.ParamBaseY = CubismDefaultParameterId.ParamBaseY;
    Live2DCubismFramework51.ParamBodyAngleX = CubismDefaultParameterId.ParamBodyAngleX;
    Live2DCubismFramework51.ParamBodyAngleY = CubismDefaultParameterId.ParamBodyAngleY;
    Live2DCubismFramework51.ParamBodyAngleZ = CubismDefaultParameterId.ParamBodyAngleZ;
    Live2DCubismFramework51.ParamBreath = CubismDefaultParameterId.ParamBreath;
    Live2DCubismFramework51.ParamBrowLAngle = CubismDefaultParameterId.ParamBrowLAngle;
    Live2DCubismFramework51.ParamBrowLForm = CubismDefaultParameterId.ParamBrowLForm;
    Live2DCubismFramework51.ParamBrowLX = CubismDefaultParameterId.ParamBrowLX;
    Live2DCubismFramework51.ParamBrowLY = CubismDefaultParameterId.ParamBrowLY;
    Live2DCubismFramework51.ParamBrowRAngle = CubismDefaultParameterId.ParamBrowRAngle;
    Live2DCubismFramework51.ParamBrowRForm = CubismDefaultParameterId.ParamBrowRForm;
    Live2DCubismFramework51.ParamBrowRX = CubismDefaultParameterId.ParamBrowRX;
    Live2DCubismFramework51.ParamBrowRY = CubismDefaultParameterId.ParamBrowRY;
    Live2DCubismFramework51.ParamBustX = CubismDefaultParameterId.ParamBustX;
    Live2DCubismFramework51.ParamBustY = CubismDefaultParameterId.ParamBustY;
    Live2DCubismFramework51.ParamCheek = CubismDefaultParameterId.ParamCheek;
    Live2DCubismFramework51.ParamEyeBallForm = CubismDefaultParameterId.ParamEyeBallForm;
    Live2DCubismFramework51.ParamEyeBallX = CubismDefaultParameterId.ParamEyeBallX;
    Live2DCubismFramework51.ParamEyeBallY = CubismDefaultParameterId.ParamEyeBallY;
    Live2DCubismFramework51.ParamEyeLOpen = CubismDefaultParameterId.ParamEyeLOpen;
    Live2DCubismFramework51.ParamEyeLSmile = CubismDefaultParameterId.ParamEyeLSmile;
    Live2DCubismFramework51.ParamEyeROpen = CubismDefaultParameterId.ParamEyeROpen;
    Live2DCubismFramework51.ParamEyeRSmile = CubismDefaultParameterId.ParamEyeRSmile;
    Live2DCubismFramework51.ParamHairBack = CubismDefaultParameterId.ParamHairBack;
    Live2DCubismFramework51.ParamHairFluffy = CubismDefaultParameterId.ParamHairFluffy;
    Live2DCubismFramework51.ParamHairFront = CubismDefaultParameterId.ParamHairFront;
    Live2DCubismFramework51.ParamHairSide = CubismDefaultParameterId.ParamHairSide;
    Live2DCubismFramework51.ParamHandL = CubismDefaultParameterId.ParamHandL;
    Live2DCubismFramework51.ParamHandR = CubismDefaultParameterId.ParamHandR;
    Live2DCubismFramework51.ParamMouthForm = CubismDefaultParameterId.ParamMouthForm;
    Live2DCubismFramework51.ParamMouthOpenY = CubismDefaultParameterId.ParamMouthOpenY;
    Live2DCubismFramework51.ParamNONE = CubismDefaultParameterId.ParamNONE;
    Live2DCubismFramework51.ParamShoulderY = CubismDefaultParameterId.ParamShoulderY;
    Live2DCubismFramework51.PartsArmLPrefix = CubismDefaultParameterId.PartsArmLPrefix;
    Live2DCubismFramework51.PartsArmPrefix = CubismDefaultParameterId.PartsArmPrefix;
    Live2DCubismFramework51.PartsArmRPrefix = CubismDefaultParameterId.PartsArmRPrefix;
    Live2DCubismFramework51.PartsIdCore = CubismDefaultParameterId.PartsIdCore;
  })(Live2DCubismFramework12 || (Live2DCubismFramework12 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/icubismmodelsetting.ts
  var ICubismModelSetting = class {
  };
  var Live2DCubismFramework13;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.ICubismModelSetting = ICubismModelSetting;
  })(Live2DCubismFramework13 || (Live2DCubismFramework13 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/cubismmodelsettingjson.ts
  var FrequestNode = /* @__PURE__ */ ((FrequestNode2) => {
    FrequestNode2[FrequestNode2["FrequestNode_Groups"] = 0] = "FrequestNode_Groups";
    FrequestNode2[FrequestNode2["FrequestNode_Moc"] = 1] = "FrequestNode_Moc";
    FrequestNode2[FrequestNode2["FrequestNode_Motions"] = 2] = "FrequestNode_Motions";
    FrequestNode2[FrequestNode2["FrequestNode_Expressions"] = 3] = "FrequestNode_Expressions";
    FrequestNode2[FrequestNode2["FrequestNode_Textures"] = 4] = "FrequestNode_Textures";
    FrequestNode2[FrequestNode2["FrequestNode_Physics"] = 5] = "FrequestNode_Physics";
    FrequestNode2[FrequestNode2["FrequestNode_Pose"] = 6] = "FrequestNode_Pose";
    FrequestNode2[FrequestNode2["FrequestNode_HitAreas"] = 7] = "FrequestNode_HitAreas";
    return FrequestNode2;
  })(FrequestNode || {});
  var CubismModelSettingJson = class extends ICubismModelSetting {
    /**
     * 引数付きコンストラクタ
     *
     * @param buffer    Model3Jsonをバイト配列として読み込んだデータバッファ
     * @param size      Model3Jsonのデータサイズ
     */
    constructor(buffer, size) {
      super();
      /**
       * Model3Jsonのキー文字列
       */
      this.version = "Version";
      this.fileReferences = "FileReferences";
      this.groups = "Groups";
      this.layout = "Layout";
      this.hitAreas = "HitAreas";
      this.moc = "Moc";
      this.textures = "Textures";
      this.physics = "Physics";
      this.pose = "Pose";
      this.expressions = "Expressions";
      this.motions = "Motions";
      this.userData = "UserData";
      this.name = "Name";
      this.filePath = "File";
      this.id = "Id";
      this.ids = "Ids";
      this.target = "Target";
      // Motions
      this.idle = "Idle";
      this.tapBody = "TapBody";
      this.pinchIn = "PinchIn";
      this.pinchOut = "PinchOut";
      this.shake = "Shake";
      this.flickHead = "FlickHead";
      this.parameter = "Parameter";
      this.soundPath = "Sound";
      this.fadeInTime = "FadeInTime";
      this.fadeOutTime = "FadeOutTime";
      // Layout
      this.centerX = "CenterX";
      this.centerY = "CenterY";
      this.x = "X";
      this.y = "Y";
      this.width = "Width";
      this.height = "Height";
      this.lipSync = "LipSync";
      this.eyeBlink = "EyeBlink";
      this.initParameter = "init_param";
      this.initPartsVisible = "init_parts_visible";
      this.val = "val";
      this._json = CubismJson.create(buffer, size);
      if (this.getJson()) {
        this._jsonValue = [
          // 順番はenum FrequestNodeと一致させる
          this.getJson().getRoot().getValueByString(this.groups),
          this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.moc),
          this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.motions),
          this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.expressions),
          this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.textures),
          this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.physics),
          this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.pose),
          this.getJson().getRoot().getValueByString(this.hitAreas)
        ];
      }
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      CubismJson.delete(this._json);
      this._jsonValue = null;
    }
    /**
     * CubismJsonオブジェクトを取得する
     *
     * @return CubismJson
     */
    getJson() {
      return this._json;
    }
    /**
     * Mocファイルの名前を取得する
     * @return Mocファイルの名前
     */
    getModelFileName() {
      if (!this.isExistModelFile()) {
        return "";
      }
      return this._jsonValue[1 /* FrequestNode_Moc */].getRawString();
    }
    /**
     * モデルが使用するテクスチャの数を取得する
     * テクスチャの数
     */
    getTextureCount() {
      if (!this.isExistTextureFiles()) {
        return 0;
      }
      return this._jsonValue[4 /* FrequestNode_Textures */].getSize();
    }
    /**
     * テクスチャが配置されたディレクトリの名前を取得する
     * @return テクスチャが配置されたディレクトリの名前
     */
    getTextureDirectory() {
      const texturePath = this._jsonValue[4 /* FrequestNode_Textures */].getValueByIndex(0).getRawString();
      const pathArray = texturePath.split("/");
      const arrayLength = pathArray.length - 1;
      let textureDirectoryStr = "";
      for (let i = 0; i < arrayLength; i++) {
        textureDirectoryStr += pathArray[i];
        if (i < arrayLength - 1) {
          textureDirectoryStr += "/";
        }
      }
      return textureDirectoryStr;
    }
    /**
     * モデルが使用するテクスチャの名前を取得する
     * @param index 配列のインデックス値
     * @return テクスチャの名前
     */
    getTextureFileName(index) {
      return this._jsonValue[4 /* FrequestNode_Textures */].getValueByIndex(index).getRawString();
    }
    /**
     * モデルに設定された当たり判定の数を取得する
     * @return モデルに設定された当たり判定の数
     */
    getHitAreasCount() {
      if (!this.isExistHitAreas()) {
        return 0;
      }
      return this._jsonValue[7 /* FrequestNode_HitAreas */].getSize();
    }
    /**
     * 当たり判定に設定されたIDを取得する
     *
     * @param index 配列のindex
     * @return 当たり判定に設定されたID
     */
    getHitAreaId(index) {
      return CubismFramework.getIdManager().getId(
        this._jsonValue[7 /* FrequestNode_HitAreas */].getValueByIndex(index).getValueByString(this.id).getRawString()
      );
    }
    /**
     * 当たり判定に設定された名前を取得する
     * @param index 配列のインデックス値
     * @return 当たり判定に設定された名前
     */
    getHitAreaName(index) {
      return this._jsonValue[7 /* FrequestNode_HitAreas */].getValueByIndex(index).getValueByString(this.name).getRawString();
    }
    /**
     * 物理演算設定ファイルの名前を取得する
     * @return 物理演算設定ファイルの名前
     */
    getPhysicsFileName() {
      if (!this.isExistPhysicsFile()) {
        return "";
      }
      return this._jsonValue[5 /* FrequestNode_Physics */].getRawString();
    }
    /**
     * パーツ切り替え設定ファイルの名前を取得する
     * @return パーツ切り替え設定ファイルの名前
     */
    getPoseFileName() {
      if (!this.isExistPoseFile()) {
        return "";
      }
      return this._jsonValue[6 /* FrequestNode_Pose */].getRawString();
    }
    /**
     * 表情設定ファイルの数を取得する
     * @return 表情設定ファイルの数
     */
    getExpressionCount() {
      if (!this.isExistExpressionFile()) {
        return 0;
      }
      return this._jsonValue[3 /* FrequestNode_Expressions */].getSize();
    }
    /**
     * 表情設定ファイルを識別する名前（別名）を取得する
     * @param index 配列のインデックス値
     * @return 表情の名前
     */
    getExpressionName(index) {
      return this._jsonValue[3 /* FrequestNode_Expressions */].getValueByIndex(index).getValueByString(this.name).getRawString();
    }
    /**
     * 表情設定ファイルの名前を取得する
     * @param index 配列のインデックス値
     * @return 表情設定ファイルの名前
     */
    getExpressionFileName(index) {
      return this._jsonValue[3 /* FrequestNode_Expressions */].getValueByIndex(index).getValueByString(this.filePath).getRawString();
    }
    /**
     * モーショングループの数を取得する
     * @return モーショングループの数
     */
    getMotionGroupCount() {
      if (!this.isExistMotionGroups()) {
        return 0;
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getKeys().length;
    }
    /**
     * モーショングループの名前を取得する
     * @param index 配列のインデックス値
     * @return モーショングループの名前
     */
    getMotionGroupName(index) {
      if (!this.isExistMotionGroups()) {
        return null;
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getKeys()[index];
    }
    /**
     * モーショングループに含まれるモーションの数を取得する
     * @param groupName モーショングループの名前
     * @return モーショングループの数
     */
    getMotionCount(groupName) {
      if (!this.isExistMotionGroupName(groupName)) {
        return 0;
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getSize();
    }
    /**
     * グループ名とインデックス値からモーションファイル名を取得する
     * @param groupName モーショングループの名前
     * @param index     配列のインデックス値
     * @return モーションファイルの名前
     */
    getMotionFileName(groupName, index) {
      if (!this.isExistMotionGroupName(groupName)) {
        return "";
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.filePath).getRawString();
    }
    /**
     * モーションに対応するサウンドファイルの名前を取得する
     * @param groupName モーショングループの名前
     * @param index 配列のインデックス値
     * @return サウンドファイルの名前
     */
    getMotionSoundFileName(groupName, index) {
      if (!this.isExistMotionSoundFile(groupName, index)) {
        return "";
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.soundPath).getRawString();
    }
    /**
     * モーション開始時のフェードイン処理時間を取得する
     * @param groupName モーショングループの名前
     * @param index 配列のインデックス値
     * @return フェードイン処理時間[秒]
     */
    getMotionFadeInTimeValue(groupName, index) {
      if (!this.isExistMotionFadeIn(groupName, index)) {
        return -1;
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.fadeInTime).toFloat();
    }
    /**
     * モーション終了時のフェードアウト処理時間を取得する
     * @param groupName モーショングループの名前
     * @param index 配列のインデックス値
     * @return フェードアウト処理時間[秒]
     */
    getMotionFadeOutTimeValue(groupName, index) {
      if (!this.isExistMotionFadeOut(groupName, index)) {
        return -1;
      }
      return this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.fadeOutTime).toFloat();
    }
    /**
     * ユーザーデータのファイル名を取得する
     * @return ユーザーデータのファイル名
     */
    getUserDataFile() {
      if (!this.isExistUserDataFile()) {
        return "";
      }
      return this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.userData).getRawString();
    }
    /**
     * レイアウト情報を取得する
     * @param outLayoutMap Mapクラスのインスタンス
     * @return true レイアウト情報が存在する
     * @return false レイアウト情報が存在しない
     */
    getLayoutMap(outLayoutMap) {
      const map = this.getJson().getRoot().getValueByString(this.layout).getMap();
      if (map == null) {
        return false;
      }
      let ret = false;
      for (const element of map) {
        outLayoutMap.set(element[0], element[1].toFloat());
        ret = true;
      }
      return ret;
    }
    /**
     * 目パチに関連付けられたパラメータの数を取得する
     * @return 目パチに関連付けられたパラメータの数
     */
    getEyeBlinkParameterCount() {
      if (!this.isExistEyeBlinkParameters()) {
        return 0;
      }
      let num = 0;
      for (let i = 0; i < this._jsonValue[0 /* FrequestNode_Groups */].getSize(); i++) {
        const refI = this._jsonValue[0 /* FrequestNode_Groups */].getValueByIndex(i);
        if (refI.isNull() || refI.isError()) {
          continue;
        }
        if (refI.getValueByString(this.name).getRawString() == this.eyeBlink) {
          num = refI.getValueByString(this.ids).getVector().length;
          break;
        }
      }
      return num;
    }
    /**
     * 目パチに関連付けられたパラメータのIDを取得する
     * @param index 配列のインデックス値
     * @return パラメータID
     */
    getEyeBlinkParameterId(index) {
      if (!this.isExistEyeBlinkParameters()) {
        return null;
      }
      for (let i = 0; i < this._jsonValue[0 /* FrequestNode_Groups */].getSize(); i++) {
        const refI = this._jsonValue[0 /* FrequestNode_Groups */].getValueByIndex(i);
        if (refI.isNull() || refI.isError()) {
          continue;
        }
        if (refI.getValueByString(this.name).getRawString() == this.eyeBlink) {
          return CubismFramework.getIdManager().getId(
            refI.getValueByString(this.ids).getValueByIndex(index).getRawString()
          );
        }
      }
      return null;
    }
    /**
     * リップシンクに関連付けられたパラメータの数を取得する
     * @return リップシンクに関連付けられたパラメータの数
     */
    getLipSyncParameterCount() {
      if (!this.isExistLipSyncParameters()) {
        return 0;
      }
      let num = 0;
      for (let i = 0; i < this._jsonValue[0 /* FrequestNode_Groups */].getSize(); i++) {
        const refI = this._jsonValue[0 /* FrequestNode_Groups */].getValueByIndex(i);
        if (refI.isNull() || refI.isError()) {
          continue;
        }
        if (refI.getValueByString(this.name).getRawString() == this.lipSync) {
          num = refI.getValueByString(this.ids).getVector().length;
          break;
        }
      }
      return num;
    }
    /**
     * リップシンクに関連付けられたパラメータの数を取得する
     * @param index 配列のインデックス値
     * @return パラメータID
     */
    getLipSyncParameterId(index) {
      if (!this.isExistLipSyncParameters()) {
        return null;
      }
      for (let i = 0; i < this._jsonValue[0 /* FrequestNode_Groups */].getSize(); i++) {
        const refI = this._jsonValue[0 /* FrequestNode_Groups */].getValueByIndex(i);
        if (refI.isNull() || refI.isError()) {
          continue;
        }
        if (refI.getValueByString(this.name).getRawString() == this.lipSync) {
          return CubismFramework.getIdManager().getId(
            refI.getValueByString(this.ids).getValueByIndex(index).getRawString()
          );
        }
      }
      return null;
    }
    /**
     * モデルファイルのキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistModelFile() {
      const node = this._jsonValue[1 /* FrequestNode_Moc */];
      return !node.isNull() && !node.isError();
    }
    /**
     * テクスチャファイルのキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistTextureFiles() {
      const node = this._jsonValue[4 /* FrequestNode_Textures */];
      return !node.isNull() && !node.isError();
    }
    /**
     * 当たり判定のキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistHitAreas() {
      const node = this._jsonValue[7 /* FrequestNode_HitAreas */];
      return !node.isNull() && !node.isError();
    }
    /**
     * 物理演算ファイルのキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistPhysicsFile() {
      const node = this._jsonValue[5 /* FrequestNode_Physics */];
      return !node.isNull() && !node.isError();
    }
    /**
     * ポーズ設定ファイルのキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistPoseFile() {
      const node = this._jsonValue[6 /* FrequestNode_Pose */];
      return !node.isNull() && !node.isError();
    }
    /**
     * 表情設定ファイルのキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistExpressionFile() {
      const node = this._jsonValue[3 /* FrequestNode_Expressions */];
      return !node.isNull() && !node.isError();
    }
    /**
     * モーショングループのキーが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistMotionGroups() {
      const node = this._jsonValue[2 /* FrequestNode_Motions */];
      return !node.isNull() && !node.isError();
    }
    /**
     * 引数で指定したモーショングループのキーが存在するかどうかを確認する
     * @param groupName  グループ名
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistMotionGroupName(groupName) {
      const node = this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(
        groupName
      );
      return !node.isNull() && !node.isError();
    }
    /**
     * 引数で指定したモーションに対応するサウンドファイルのキーが存在するかどうかを確認する
     * @param groupName  グループ名
     * @param index 配列のインデックス値
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistMotionSoundFile(groupName, index) {
      const node = this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.soundPath);
      return !node.isNull() && !node.isError();
    }
    /**
     * 引数で指定したモーションに対応するフェードイン時間のキーが存在するかどうかを確認する
     * @param groupName  グループ名
     * @param index 配列のインデックス値
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistMotionFadeIn(groupName, index) {
      const node = this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.fadeInTime);
      return !node.isNull() && !node.isError();
    }
    /**
     * 引数で指定したモーションに対応するフェードアウト時間のキーが存在するかどうかを確認する
     * @param groupName  グループ名
     * @param index 配列のインデックス値
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistMotionFadeOut(groupName, index) {
      const node = this._jsonValue[2 /* FrequestNode_Motions */].getValueByString(groupName).getValueByIndex(index).getValueByString(this.fadeOutTime);
      return !node.isNull() && !node.isError();
    }
    /**
     * UserDataのファイル名が存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistUserDataFile() {
      const node = this.getJson().getRoot().getValueByString(this.fileReferences).getValueByString(this.userData);
      return !node.isNull() && !node.isError();
    }
    /**
     * 目ぱちに対応付けられたパラメータが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistEyeBlinkParameters() {
      if (this._jsonValue[0 /* FrequestNode_Groups */].isNull() || this._jsonValue[0 /* FrequestNode_Groups */].isError()) {
        return false;
      }
      for (let i = 0; i < this._jsonValue[0 /* FrequestNode_Groups */].getSize(); ++i) {
        if (this._jsonValue[0 /* FrequestNode_Groups */].getValueByIndex(i).getValueByString(this.name).getRawString() == this.eyeBlink) {
          return true;
        }
      }
      return false;
    }
    /**
     * リップシンクに対応付けられたパラメータが存在するかどうかを確認する
     * @return true キーが存在する
     * @return false キーが存在しない
     */
    isExistLipSyncParameters() {
      if (this._jsonValue[0 /* FrequestNode_Groups */].isNull() || this._jsonValue[0 /* FrequestNode_Groups */].isError()) {
        return false;
      }
      for (let i = 0; i < this._jsonValue[0 /* FrequestNode_Groups */].getSize(); ++i) {
        if (this._jsonValue[0 /* FrequestNode_Groups */].getValueByIndex(i).getValueByString(this.name).getRawString() == this.lipSync) {
          return true;
        }
      }
      return false;
    }
  };
  var Live2DCubismFramework14;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismModelSettingJson = CubismModelSettingJson;
    Live2DCubismFramework51.FrequestNode = FrequestNode;
  })(Live2DCubismFramework14 || (Live2DCubismFramework14 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/effect/cubismbreath.ts
  var CubismBreath = class _CubismBreath {
    /**
     * インスタンスの作成
     */
    static create() {
      return new _CubismBreath();
    }
    /**
     * インスタンスの破棄
     * @param instance 対象のCubismBreath
     */
    static delete(instance) {
      if (instance != null) {
        instance = null;
      }
    }
    /**
     * 呼吸のパラメータの紐づけ
     * @param breathParameters 呼吸を紐づけたいパラメータのリスト
     */
    setParameters(breathParameters) {
      this._breathParameters = breathParameters;
    }
    /**
     * 呼吸に紐づいているパラメータの取得
     * @return 呼吸に紐づいているパラメータのリスト
     */
    getParameters() {
      return this._breathParameters;
    }
    /**
     * モデルのパラメータの更新
     * @param model 対象のモデル
     * @param deltaTimeSeconds デルタ時間[秒]
     */
    updateParameters(model, deltaTimeSeconds) {
      this._currentTime += deltaTimeSeconds;
      const t = this._currentTime * 2 * Math.PI;
      for (let i = 0; i < this._breathParameters.length; ++i) {
        const data = this._breathParameters[i];
        model.addParameterValueById(
          data.parameterId,
          data.offset + data.peak * Math.sin(t / data.cycle),
          data.weight
        );
      }
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._currentTime = 0;
    }
    // 積算時間[秒]
  };
  var BreathParameterData = class {
    /**
     * コンストラクタ
     * @param parameterId   呼吸をひもづけるパラメータID
     * @param offset        呼吸を正弦波としたときの、波のオフセット
     * @param peak          呼吸を正弦波としたときの、波の高さ
     * @param cycle         呼吸を正弦波としたときの、波の周期
     * @param weight        パラメータへの重み
     */
    constructor(parameterId, offset, peak, cycle, weight) {
      this.parameterId = parameterId == void 0 ? null : parameterId;
      this.offset = offset == void 0 ? 0 : offset;
      this.peak = peak == void 0 ? 0 : peak;
      this.cycle = cycle == void 0 ? 0 : cycle;
      this.weight = weight == void 0 ? 0 : weight;
    }
    // パラメータへの重み
  };
  var Live2DCubismFramework15;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.BreathParameterData = BreathParameterData;
    Live2DCubismFramework51.CubismBreath = CubismBreath;
  })(Live2DCubismFramework15 || (Live2DCubismFramework15 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/effect/cubismlook.ts
  var CubismLook = class _CubismLook {
    /**
     * インスタンスの作成
     */
    static create() {
      return new _CubismLook();
    }
    /**
     * インスタンスの破棄
     * @param instance 対象のCubismDrag
     */
    static delete(instance) {
      if (instance != null) {
        instance = null;
      }
    }
    /**
     * ターゲット追従のパラメータの紐づけ
     * @param lookParameters ターゲット追従を紐づけたいパラメータのリスト
     */
    setParameters(lookParameters) {
      this._lookParameters = lookParameters;
    }
    /**
     * ターゲット追従に紐づいているパラメータの取得
     * @return ターゲット追従に紐づいているパラメータのリスト
     */
    getParameters() {
      return this._lookParameters;
    }
    /**
     * モデルのパラメータの更新
     * @param model 対象のモデル
     * @param dragX ターゲットのX座標
     * @param dragY ターゲットのY座標
     */
    updateParameters(model, dragX, dragY) {
      for (let i = 0; i < this._lookParameters.length; ++i) {
        const data = this._lookParameters[i];
        model.addParameterValueById(
          data.parameterId,
          data.factorX * dragX + data.factorY * dragY + data.factorXY * dragX * dragY
        );
      }
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._lookParameters = new Array();
    }
    // ターゲット追従に紐づいているパラメータのリスト
  };
  var LookParameterData = class {
    /**
     * コンストラクタ
     * @param parameterId   ターゲット追従を紐づけるパラメータID
     * @param factorX       X方向ドラッグ入力に対する係数
     * @param factorY       Y方向ドラッグ入力に対する係数
     * @param factorXY      XY積ドラッグ入力に対する係数
     */
    constructor(parameterId, factorX, factorY, factorXY) {
      this.parameterId = parameterId == void 0 ? null : parameterId;
      this.factorX = factorX == void 0 ? 0 : factorX;
      this.factorY = factorY == void 0 ? 0 : factorY;
      this.factorXY = factorXY == void 0 ? 0 : factorXY;
    }
    // XY積ドラッグ入力に対する係数
  };
  var Live2DCubismFramework16;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.LookParameterData = LookParameterData;
    Live2DCubismFramework51.CubismLook = CubismLook;
  })(Live2DCubismFramework16 || (Live2DCubismFramework16 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/effect/cubismeyeblink.ts
  var _CubismEyeBlink = class _CubismEyeBlink {
    /**
     * インスタンスを作成する
     * @param modelSetting モデルの設定情報
     * @return 作成されたインスタンス
     * @note 引数がNULLの場合、パラメータIDが設定されていない空のインスタンスを作成する。
     */
    static create(modelSetting = null) {
      return new _CubismEyeBlink(modelSetting);
    }
    /**
     * インスタンスの破棄
     * @param eyeBlink 対象のCubismEyeBlink
     */
    static delete(eyeBlink) {
      if (eyeBlink != null) {
        eyeBlink = null;
      }
    }
    /**
     * まばたきの間隔の設定
     * @param blinkingInterval まばたきの間隔の時間[秒]
     */
    setBlinkingInterval(blinkingInterval) {
      this._blinkingIntervalSeconds = blinkingInterval;
    }
    /**
     * まばたきのモーションの詳細設定
     * @param closing   まぶたを閉じる動作の所要時間[秒]
     * @param closed    まぶたを閉じている動作の所要時間[秒]
     * @param opening   まぶたを開く動作の所要時間[秒]
     */
    setBlinkingSetting(closing, closed, opening) {
      this._closingSeconds = closing;
      this._closedSeconds = closed;
      this._openingSeconds = opening;
    }
    /**
     * まばたきさせるパラメータIDのリストの設定
     * @param parameterIds パラメータのIDのリスト
     */
    setParameterIds(parameterIds) {
      this._parameterIds = parameterIds;
    }
    /**
     * まばたきさせるパラメータIDのリストの取得
     * @return パラメータIDのリスト
     */
    getParameterIds() {
      return this._parameterIds;
    }
    /**
     * モデルのパラメータの更新
     * @param model 対象のモデル
     * @param deltaTimeSeconds デルタ時間[秒]
     */
    updateParameters(model, deltaTimeSeconds) {
      this._userTimeSeconds += deltaTimeSeconds;
      let parameterValue;
      let t = 0;
      const blinkingState = this._blinkingState;
      switch (blinkingState) {
        case 2 /* EyeState_Closing */:
          t = (this._userTimeSeconds - this._stateStartTimeSeconds) / this._closingSeconds;
          if (t >= 1) {
            t = 1;
            this._blinkingState = 3 /* EyeState_Closed */;
            this._stateStartTimeSeconds = this._userTimeSeconds;
          }
          parameterValue = 1 - t;
          break;
        case 3 /* EyeState_Closed */:
          t = (this._userTimeSeconds - this._stateStartTimeSeconds) / this._closedSeconds;
          if (t >= 1) {
            this._blinkingState = 4 /* EyeState_Opening */;
            this._stateStartTimeSeconds = this._userTimeSeconds;
          }
          parameterValue = 0;
          break;
        case 4 /* EyeState_Opening */:
          t = (this._userTimeSeconds - this._stateStartTimeSeconds) / this._openingSeconds;
          if (t >= 1) {
            t = 1;
            this._blinkingState = 1 /* EyeState_Interval */;
            this._nextBlinkingTime = this.determinNextBlinkingTiming();
          }
          parameterValue = t;
          break;
        case 1 /* EyeState_Interval */:
          if (this._nextBlinkingTime < this._userTimeSeconds) {
            this._blinkingState = 2 /* EyeState_Closing */;
            this._stateStartTimeSeconds = this._userTimeSeconds;
          }
          parameterValue = 1;
          break;
        case 0 /* EyeState_First */:
        default:
          this._blinkingState = 1 /* EyeState_Interval */;
          this._nextBlinkingTime = this.determinNextBlinkingTiming();
          parameterValue = 1;
          break;
      }
      if (!_CubismEyeBlink.CloseIfZero) {
        parameterValue = -parameterValue;
      }
      for (let i = 0; i < this._parameterIds.length; ++i) {
        model.setParameterValueById(this._parameterIds[i], parameterValue);
      }
    }
    /**
     * コンストラクタ
     * @param modelSetting モデルの設定情報
     */
    constructor(modelSetting) {
      this._blinkingState = 0 /* EyeState_First */;
      this._nextBlinkingTime = 0;
      this._stateStartTimeSeconds = 0;
      this._blinkingIntervalSeconds = 4;
      this._closingSeconds = 0.1;
      this._closedSeconds = 0.05;
      this._openingSeconds = 0.15;
      this._userTimeSeconds = 0;
      this._parameterIds = new Array();
      if (modelSetting == null) {
        return;
      }
      this._parameterIds.length = modelSetting.getEyeBlinkParameterCount();
      for (let i = 0; i < modelSetting.getEyeBlinkParameterCount(); ++i) {
        this._parameterIds[i] = modelSetting.getEyeBlinkParameterId(i);
      }
    }
    /**
     * 次の瞬きのタイミングの決定
     *
     * @return 次のまばたきを行う時刻[秒]
     */
    determinNextBlinkingTiming() {
      const r = Math.random();
      return this._userTimeSeconds + r * (2 * this._blinkingIntervalSeconds - 1);
    }
  };
  // デルタ時間の積算値[秒]
  /**
   * IDで指定された目のパラメータが、0のときに閉じるなら true 、1の時に閉じるなら false 。
   */
  _CubismEyeBlink.CloseIfZero = true;
  var CubismEyeBlink = _CubismEyeBlink;
  var EyeState = /* @__PURE__ */ ((EyeState2) => {
    EyeState2[EyeState2["EyeState_First"] = 0] = "EyeState_First";
    EyeState2[EyeState2["EyeState_Interval"] = 1] = "EyeState_Interval";
    EyeState2[EyeState2["EyeState_Closing"] = 2] = "EyeState_Closing";
    EyeState2[EyeState2["EyeState_Closed"] = 3] = "EyeState_Closed";
    EyeState2[EyeState2["EyeState_Opening"] = 4] = "EyeState_Opening";
    return EyeState2;
  })(EyeState || {});
  var Live2DCubismFramework17;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismEyeBlink = CubismEyeBlink;
    Live2DCubismFramework51.EyeState = EyeState;
  })(Live2DCubismFramework17 || (Live2DCubismFramework17 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/effect/cubismpose.ts
  var Epsilon = 1e-3;
  var DefaultFadeInSeconds = 0.5;
  var FadeIn = "FadeInTime";
  var Link = "Link";
  var Groups = "Groups";
  var Id = "Id";
  var CubismPose = class _CubismPose {
    /**
     * インスタンスの作成
     * @param pose3json pose3.jsonのデータ
     * @param size pose3.jsonのデータのサイズ[byte]
     * @return 作成されたインスタンス
     */
    static create(pose3json, size) {
      const json = CubismJson.create(pose3json, size);
      if (!json) {
        return null;
      }
      const ret = new _CubismPose();
      const root = json.getRoot();
      if (!root.getValueByString(FadeIn).isNull()) {
        ret._fadeTimeSeconds = root.getValueByString(FadeIn).toFloat(DefaultFadeInSeconds);
        if (ret._fadeTimeSeconds < 0) {
          ret._fadeTimeSeconds = DefaultFadeInSeconds;
        }
      }
      const poseListInfo = root.getValueByString(Groups);
      const poseCount = poseListInfo.getSize();
      ret._partGroupCounts.length = poseCount;
      for (let poseIndex = 0; poseIndex < poseCount; ++poseIndex) {
        const idListInfo = poseListInfo.getValueByIndex(poseIndex);
        const idCount = idListInfo.getSize();
        let groupCount = 0;
        for (let groupIndex = 0; groupIndex < idCount; ++groupIndex) {
          const partInfo = idListInfo.getValueByIndex(groupIndex);
          const partData = new PartData();
          const parameterId = CubismFramework.getIdManager().getId(
            partInfo.getValueByString(Id).getRawString()
          );
          partData.partId = parameterId;
          if (!partInfo.getValueByString(Link).isNull()) {
            const linkListInfo = partInfo.getValueByString(Link);
            const linkCount = linkListInfo.getSize();
            for (let linkIndex = 0; linkIndex < linkCount; ++linkIndex) {
              const linkPart = new PartData();
              const linkId = CubismFramework.getIdManager().getId(
                linkListInfo.getValueByIndex(linkIndex).getString()
              );
              linkPart.partId = linkId;
              partData.link.push(linkPart);
            }
          }
          ret._partGroups.push(partData.clone());
          ++groupCount;
        }
        ret._partGroupCounts[poseIndex] = groupCount;
      }
      CubismJson.delete(json);
      return ret;
    }
    /**
     * インスタンスを破棄する
     * @param pose 対象のCubismPose
     */
    static delete(pose) {
      if (pose != null) {
        pose = null;
      }
    }
    /**
     * モデルのパラメータの更新
     * @param model 対象のモデル
     * @param deltaTimeSeconds デルタ時間[秒]
     */
    updateParameters(model, deltaTimeSeconds) {
      if (model != this._lastModel) {
        this.reset(model);
      }
      this._lastModel = model;
      if (deltaTimeSeconds < 0) {
        deltaTimeSeconds = 0;
      }
      let beginIndex = 0;
      for (let i = 0; i < this._partGroupCounts.length; i++) {
        const partGroupCount = this._partGroupCounts[i];
        this.doFade(model, deltaTimeSeconds, beginIndex, partGroupCount);
        beginIndex += partGroupCount;
      }
      this.copyPartOpacities(model);
    }
    /**
     * 表示を初期化
     * @param model 対象のモデル
     * @note 不透明度の初期値が0でないパラメータは、不透明度を１に設定する
     */
    reset(model) {
      let beginIndex = 0;
      for (let i = 0; i < this._partGroupCounts.length; ++i) {
        const groupCount = this._partGroupCounts[i];
        for (let j = beginIndex; j < beginIndex + groupCount; ++j) {
          this._partGroups[j].initialize(model);
          const partsIndex = this._partGroups[j].partIndex;
          const paramIndex = this._partGroups[j].parameterIndex;
          if (partsIndex < 0) {
            continue;
          }
          model.setPartOpacityByIndex(partsIndex, j == beginIndex ? 1 : 0);
          model.setParameterValueByIndex(paramIndex, j == beginIndex ? 1 : 0);
          for (let k = 0; k < this._partGroups[j].link.length; ++k) {
            this._partGroups[j].link[k].initialize(model);
          }
        }
        beginIndex += groupCount;
      }
    }
    /**
     * パーツの不透明度をコピー
     *
     * @param model 対象のモデル
     */
    copyPartOpacities(model) {
      for (let groupIndex = 0; groupIndex < this._partGroups.length; ++groupIndex) {
        const partData = this._partGroups[groupIndex];
        if (partData.link.length == 0) {
          continue;
        }
        const partIndex = this._partGroups[groupIndex].partIndex;
        const opacity = model.getPartOpacityByIndex(partIndex);
        for (let linkIndex = 0; linkIndex < partData.link.length; ++linkIndex) {
          const linkPart = partData.link[linkIndex];
          const linkPartIndex = linkPart.partIndex;
          if (linkPartIndex < 0) {
            continue;
          }
          model.setPartOpacityByIndex(linkPartIndex, opacity);
        }
      }
    }
    /**
     * パーツのフェード操作を行う。
     * @param model 対象のモデル
     * @param deltaTimeSeconds デルタ時間[秒]
     * @param beginIndex フェード操作を行うパーツグループの先頭インデックス
     * @param partGroupCount フェード操作を行うパーツグループの個数
     */
    doFade(model, deltaTimeSeconds, beginIndex, partGroupCount) {
      let visiblePartIndex = -1;
      let newOpacity = 1;
      const phi = 0.5;
      const backOpacityThreshold = 0.15;
      for (let i = beginIndex; i < beginIndex + partGroupCount; ++i) {
        const partIndex = this._partGroups[i].partIndex;
        const paramIndex = this._partGroups[i].parameterIndex;
        if (model.getParameterValueByIndex(paramIndex) > Epsilon) {
          if (visiblePartIndex >= 0) {
            break;
          }
          visiblePartIndex = i;
          if (this._fadeTimeSeconds == 0) {
            newOpacity = 1;
            continue;
          }
          newOpacity = model.getPartOpacityByIndex(partIndex);
          newOpacity += deltaTimeSeconds / this._fadeTimeSeconds;
          if (newOpacity > 1) {
            newOpacity = 1;
          }
        }
      }
      if (visiblePartIndex < 0) {
        visiblePartIndex = 0;
        newOpacity = 1;
      }
      for (let i = beginIndex; i < beginIndex + partGroupCount; ++i) {
        const partsIndex = this._partGroups[i].partIndex;
        if (visiblePartIndex == i) {
          model.setPartOpacityByIndex(partsIndex, newOpacity);
        } else {
          let opacity = model.getPartOpacityByIndex(partsIndex);
          let a1;
          if (newOpacity < phi) {
            a1 = newOpacity * (phi - 1) / phi + 1;
          } else {
            a1 = (1 - newOpacity) * phi / (1 - phi);
          }
          const backOpacity = (1 - a1) * (1 - newOpacity);
          if (backOpacity > backOpacityThreshold) {
            a1 = 1 - backOpacityThreshold / (1 - newOpacity);
          }
          if (opacity > a1) {
            opacity = a1;
          }
          model.setPartOpacityByIndex(partsIndex, opacity);
        }
      }
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._fadeTimeSeconds = DefaultFadeInSeconds;
      this._lastModel = null;
      this._partGroups = new Array();
      this._partGroupCounts = new Array();
    }
    // 前回操作したモデル
  };
  var PartData = class _PartData {
    /**
     * コンストラクタ
     */
    constructor(v) {
      this.parameterIndex = 0;
      this.partIndex = 0;
      this.link = new Array();
      if (v != void 0) {
        this.partId = v.partId;
        this.link.length = v.link.length;
        for (let i = 0; i < v.link.length; i++) {
          this.link[i] = v.link[i].clone();
        }
      }
    }
    /**
     * =演算子のオーバーロード
     */
    assignment(v) {
      this.partId = v.partId;
      let dstIndex = this.link.length;
      this.link.length += v.link.length;
      for (const partData of v.link) {
        this.link[dstIndex++] = partData.clone();
      }
      return this;
    }
    /**
     * 初期化
     * @param model 初期化に使用するモデル
     */
    initialize(model) {
      this.parameterIndex = model.getParameterIndex(this.partId);
      this.partIndex = model.getPartIndex(this.partId);
      model.setParameterValueByIndex(this.parameterIndex, 1);
    }
    /**
     * オブジェクトのコピーを生成する
     */
    clone() {
      const clonePartData = new _PartData();
      clonePartData.partId = this.partId;
      clonePartData.parameterIndex = this.parameterIndex;
      clonePartData.partIndex = this.partIndex;
      clonePartData.link = new Array();
      clonePartData.link.length = this.link.length;
      for (let i = 0; i < this.link.length; i++) {
        clonePartData.link[i] = this.link[i].clone();
      }
      return clonePartData;
    }
    // 連動するパラメータ
  };
  var Live2DCubismFramework18;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismPose = CubismPose;
    Live2DCubismFramework51.PartData = PartData;
  })(Live2DCubismFramework18 || (Live2DCubismFramework18 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/math/cubismmodelmatrix.ts
  var CubismModelMatrix = class extends CubismMatrix44 {
    /**
     * コンストラクタ
     *
     * @param w 横幅
     * @param h 縦幅
     */
    constructor(w, h) {
      super();
      this._width = w !== void 0 ? w : 0;
      this._height = h !== void 0 ? h : 0;
      this.setHeight(2);
    }
    /**
     * 横幅を設定
     *
     * @param w 横幅
     */
    setWidth(w) {
      const scaleX = w / this._width;
      const scaleY = scaleX;
      this.scale(scaleX, scaleY);
    }
    /**
     * 縦幅を設定
     * @param h 縦幅
     */
    setHeight(h) {
      const scaleX = h / this._height;
      const scaleY = scaleX;
      this.scale(scaleX, scaleY);
    }
    /**
     * 位置を設定
     *
     * @param x X軸の位置
     * @param y Y軸の位置
     */
    setPosition(x, y) {
      this.translate(x, y);
    }
    /**
     * 中心位置を設定
     *
     * @param x X軸の中心位置
     * @param y Y軸の中心位置
     *
     * @note widthかheightを設定したあとでないと、拡大率が正しく取得できないためずれる。
     */
    setCenterPosition(x, y) {
      this.centerX(x);
      this.centerY(y);
    }
    /**
     * 上辺の位置を設定する
     *
     * @param y 上辺のY軸位置
     */
    top(y) {
      this.setY(y);
    }
    /**
     * 下辺の位置を設定する
     *
     * @param y 下辺のY軸位置
     */
    bottom(y) {
      const h = this._height * this.getScaleY();
      this.translateY(y - h);
    }
    /**
     * 左辺の位置を設定
     *
     * @param x 左辺のX軸位置
     */
    left(x) {
      this.setX(x);
    }
    /**
     * 右辺の位置を設定
     *
     * @param x 右辺のX軸位置
     */
    right(x) {
      const w = this._width * this.getScaleX();
      this.translateX(x - w);
    }
    /**
     * X軸の中心位置を設定
     *
     * @param x X軸の中心位置
     */
    centerX(x) {
      const w = this._width * this.getScaleX();
      this.translateX(x - w / 2);
    }
    /**
     * X軸の位置を設定
     *
     * @param x X軸の位置
     */
    setX(x) {
      this.translateX(x);
    }
    /**
     * Y軸の中心位置を設定
     *
     * @param y Y軸の中心位置
     */
    centerY(y) {
      const h = this._height * this.getScaleY();
      this.translateY(y - h / 2);
    }
    /**
     * Y軸の位置を設定する
     *
     * @param y Y軸の位置
     */
    setY(y) {
      this.translateY(y);
    }
    /**
     * レイアウト情報から位置を設定
     *
     * @param layout レイアウト情報
     */
    setupFromLayout(layout) {
      const keyWidth = "width";
      const keyHeight = "height";
      const keyX = "x";
      const keyY = "y";
      const keyCenterX = "center_x";
      const keyCenterY = "center_y";
      const keyTop = "top";
      const keyBottom = "bottom";
      const keyLeft = "left";
      const keyRight = "right";
      for (const item of layout) {
        const key = item[0];
        const value = item[1];
        if (key == keyWidth) {
          this.setWidth(value);
        } else if (key == keyHeight) {
          this.setHeight(value);
        }
      }
      for (const item of layout) {
        const key = item[0];
        const value = item[1];
        if (key == keyX) {
          this.setX(value);
        } else if (key == keyY) {
          this.setY(value);
        } else if (key == keyCenterX) {
          this.centerX(value);
        } else if (key == keyCenterY) {
          this.centerY(value);
        } else if (key == keyTop) {
          this.top(value);
        } else if (key == keyBottom) {
          this.bottom(value);
        } else if (key == keyLeft) {
          this.left(value);
        } else if (key == keyRight) {
          this.right(value);
        }
      }
    }
    // 縦幅
  };
  var Live2DCubismFramework19;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismModelMatrix = CubismModelMatrix;
  })(Live2DCubismFramework19 || (Live2DCubismFramework19 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/math/cubismtargetpoint.ts
  var FrameRate = 30;
  var Epsilon2 = 0.01;
  var CubismTargetPoint = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._faceTargetX = 0;
      this._faceTargetY = 0;
      this._faceX = 0;
      this._faceY = 0;
      this._faceVX = 0;
      this._faceVY = 0;
      this._lastTimeSeconds = 0;
      this._userTimeSeconds = 0;
    }
    /**
     * 更新処理
     */
    update(deltaTimeSeconds) {
      this._userTimeSeconds += deltaTimeSeconds;
      const faceParamMaxV = 40 / 10;
      const maxV = faceParamMaxV * 1 / FrameRate;
      if (this._lastTimeSeconds == 0) {
        this._lastTimeSeconds = this._userTimeSeconds;
        return;
      }
      const deltaTimeWeight = (this._userTimeSeconds - this._lastTimeSeconds) * FrameRate;
      this._lastTimeSeconds = this._userTimeSeconds;
      const timeToMaxSpeed = 0.15;
      const frameToMaxSpeed = timeToMaxSpeed * FrameRate;
      const maxA = deltaTimeWeight * maxV / frameToMaxSpeed;
      const dx = this._faceTargetX - this._faceX;
      const dy = this._faceTargetY - this._faceY;
      if (CubismMath.abs(dx) <= Epsilon2 && CubismMath.abs(dy) <= Epsilon2) {
        return;
      }
      const d = CubismMath.sqrt(dx * dx + dy * dy);
      const vx = maxV * dx / d;
      const vy = maxV * dy / d;
      let ax = vx - this._faceVX;
      let ay = vy - this._faceVY;
      const a = CubismMath.sqrt(ax * ax + ay * ay);
      if (a < -maxA || a > maxA) {
        ax *= maxA / a;
        ay *= maxA / a;
      }
      this._faceVX += ax;
      this._faceVY += ay;
      {
        const maxV2 = 0.5 * (CubismMath.sqrt(maxA * maxA + 16 * maxA * d - 8 * maxA * d) - maxA);
        const curV = CubismMath.sqrt(
          this._faceVX * this._faceVX + this._faceVY * this._faceVY
        );
        if (curV > maxV2) {
          this._faceVX *= maxV2 / curV;
          this._faceVY *= maxV2 / curV;
        }
      }
      this._faceX += this._faceVX;
      this._faceY += this._faceVY;
    }
    /**
     * X軸の顔の向きの値を取得
     *
     * @return X軸の顔の向きの値（-1.0 ~ 1.0）
     */
    getX() {
      return this._faceX;
    }
    /**
     * Y軸の顔の向きの値を取得
     *
     * @return Y軸の顔の向きの値（-1.0 ~ 1.0）
     */
    getY() {
      return this._faceY;
    }
    /**
     * 顔の向きの目標値を設定
     *
     * @param x X軸の顔の向きの値（-1.0 ~ 1.0）
     * @param y Y軸の顔の向きの値（-1.0 ~ 1.0）
     */
    set(x, y) {
      this._faceTargetX = x;
      this._faceTargetY = y;
    }
    // デルタ時間の積算値[秒]
  };
  var Live2DCubismFramework20;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismTargetPoint = CubismTargetPoint;
  })(Live2DCubismFramework20 || (Live2DCubismFramework20 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/acubismmotion.ts
  var ACubismMotion = class {
    /**
     * コンストラクタ
     */
    constructor() {
      /**
       * モーション再生開始コールバックの登録
       *
       * モーション再生開始コールバックを登録する。
       * 以下の状態の際には呼び出されない:
       *   1. 再生中のモーションが「ループ」として設定されているとき
       *   2. コールバックが登録されていない時
       *
       * @param onBeganMotionHandler モーション再生開始コールバック関数
       */
      this.setBeganMotionHandler = (onBeganMotionHandler) => this._onBeganMotion = onBeganMotionHandler;
      /**
       * モーション再生開始コールバックの取得
       *
       * モーション再生開始コールバックを取得する。
       *
       * @return 登録されているモーション再生開始コールバック関数
       */
      this.getBeganMotionHandler = () => this._onBeganMotion;
      /**
       * モーション再生終了コールバックの登録
       *
       * モーション再生終了コールバックを登録する。
       * isFinishedフラグを設定するタイミングで呼び出される。
       * 以下の状態の際には呼び出されない:
       *   1. 再生中のモーションが「ループ」として設定されているとき
       *   2. コールバックが登録されていない時
       *
       * @param onFinishedMotionHandler モーション再生終了コールバック関数
       */
      this.setFinishedMotionHandler = (onFinishedMotionHandler) => this._onFinishedMotion = onFinishedMotionHandler;
      /**
       * モーション再生終了コールバックの取得
       *
       * モーション再生終了コールバックを取得する。
       *
       * @return 登録されているモーション再生終了コールバック関数
       */
      this.getFinishedMotionHandler = () => this._onFinishedMotion;
      this._fadeInSeconds = -1;
      this._fadeOutSeconds = -1;
      this._weight = 1;
      this._offsetSeconds = 0;
      this._isLoop = false;
      this._isLoopFadeIn = true;
      this._previousLoopState = this._isLoop;
      this._firedEventValues = new Array();
    }
    /**
     * インスタンスの破棄
     */
    static delete(motion) {
      motion.release();
      motion = null;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this._weight = 0;
    }
    /**
     * モデルのパラメータ
     * @param model 対象のモデル
     * @param motionQueueEntry CubismMotionQueueManagerで管理されているモーション
     * @param userTimeSeconds デルタ時間の積算値[秒]
     */
    updateParameters(model, motionQueueEntry, userTimeSeconds) {
      if (!motionQueueEntry.isAvailable() || motionQueueEntry.isFinished()) {
        return;
      }
      this.setupMotionQueueEntry(motionQueueEntry, userTimeSeconds);
      const fadeWeight = this.updateFadeWeight(motionQueueEntry, userTimeSeconds);
      this.doUpdateParameters(
        model,
        userTimeSeconds,
        fadeWeight,
        motionQueueEntry
      );
      if (motionQueueEntry.getEndTime() > 0 && motionQueueEntry.getEndTime() < userTimeSeconds) {
        motionQueueEntry.setIsFinished(true);
      }
    }
    /**
     * @brief モデルの再生開始処理
     *
     * モーションの再生を開始するためのセットアップを行う。
     *
     * @param[in]   motionQueueEntry    CubismMotionQueueManagerで管理されているモーション
     * @param[in]   userTimeSeconds     デルタ時間の積算値[秒]
     */
    setupMotionQueueEntry(motionQueueEntry, userTimeSeconds) {
      if (motionQueueEntry == null || motionQueueEntry.isStarted()) {
        return;
      }
      if (!motionQueueEntry.isAvailable()) {
        return;
      }
      motionQueueEntry.setIsStarted(true);
      motionQueueEntry.setStartTime(userTimeSeconds - this._offsetSeconds);
      motionQueueEntry.setFadeInStartTime(userTimeSeconds);
      if (motionQueueEntry.getEndTime() < 0) {
        this.adjustEndTime(motionQueueEntry);
      }
      if (motionQueueEntry._motion._onBeganMotion) {
        motionQueueEntry._motion._onBeganMotion(motionQueueEntry._motion);
      }
    }
    /**
     * @brief モデルのウェイト更新
     *
     * モーションのウェイトを更新する。
     *
     * @param[in]   motionQueueEntry    CubismMotionQueueManagerで管理されているモーション
     * @param[in]   userTimeSeconds     デルタ時間の積算値[秒]
     */
    updateFadeWeight(motionQueueEntry, userTimeSeconds) {
      if (motionQueueEntry == null) {
        CubismDebug.print(4 /* LogLevel_Error */, "motionQueueEntry is null.");
      }
      let fadeWeight = this._weight;
      const fadeIn = this._fadeInSeconds == 0 ? 1 : CubismMath.getEasingSine(
        (userTimeSeconds - motionQueueEntry.getFadeInStartTime()) / this._fadeInSeconds
      );
      const fadeOut = this._fadeOutSeconds == 0 || motionQueueEntry.getEndTime() < 0 ? 1 : CubismMath.getEasingSine(
        (motionQueueEntry.getEndTime() - userTimeSeconds) / this._fadeOutSeconds
      );
      fadeWeight = fadeWeight * fadeIn * fadeOut;
      motionQueueEntry.setState(userTimeSeconds, fadeWeight);
      CSM_ASSERT(0 <= fadeWeight && fadeWeight <= 1);
      return fadeWeight;
    }
    /**
     * フェードインの時間を設定する
     * @param fadeInSeconds フェードインにかかる時間[秒]
     */
    setFadeInTime(fadeInSeconds) {
      this._fadeInSeconds = fadeInSeconds;
    }
    /**
     * フェードアウトの時間を設定する
     * @param fadeOutSeconds フェードアウトにかかる時間[秒]
     */
    setFadeOutTime(fadeOutSeconds) {
      this._fadeOutSeconds = fadeOutSeconds;
    }
    /**
     * フェードアウトにかかる時間の取得
     * @return フェードアウトにかかる時間[秒]
     */
    getFadeOutTime() {
      return this._fadeOutSeconds;
    }
    /**
     * フェードインにかかる時間の取得
     * @return フェードインにかかる時間[秒]
     */
    getFadeInTime() {
      return this._fadeInSeconds;
    }
    /**
     * モーション適用の重みの設定
     * @param weight 重み（0.0 - 1.0）
     */
    setWeight(weight) {
      this._weight = weight;
    }
    /**
     * モーション適用の重みの取得
     * @return 重み（0.0 - 1.0）
     */
    getWeight() {
      return this._weight;
    }
    /**
     * モーションの長さの取得
     * @return モーションの長さ[秒]
     *
     * @note ループの時は「-1」。
     *       ループでない場合は、オーバーライドする。
     *       正の値の時は取得される時間で終了する。
     *       「-1」の時は外部から停止命令がない限り終わらない処理となる。
     */
    getDuration() {
      return -1;
    }
    /**
     * モーションのループ1回分の長さの取得
     * @return モーションのループ一回分の長さ[秒]
     *
     * @note ループしない場合は、getDuration()と同じ値を返す
     *       ループ一回分の長さが定義できない場合(プログラム的に動き続けるサブクラスなど)の場合は「-1」を返す
     */
    getLoopDuration() {
      return -1;
    }
    /**
     * モーション再生の開始時刻の設定
     * @param offsetSeconds モーション再生の開始時刻[秒]
     */
    setOffsetTime(offsetSeconds) {
      this._offsetSeconds = offsetSeconds;
    }
    /**
     * ループ情報の設定
     * @param loop ループ情報
     */
    setLoop(loop) {
      this._isLoop = loop;
    }
    /**
     * ループ情報の取得
     * @return true ループする
     * @return false ループしない
     */
    getLoop() {
      return this._isLoop;
    }
    /**
     * ループ時のフェードイン情報の設定
     * @param loopFadeIn  ループ時のフェードイン情報
     */
    setLoopFadeIn(loopFadeIn) {
      this._isLoopFadeIn = loopFadeIn;
    }
    /**
     * ループ時のフェードイン情報の取得
     *
     * @return  true    する
     * @return  false   しない
     */
    getLoopFadeIn() {
      return this._isLoopFadeIn;
    }
    /**
     * モデルのパラメータ更新
     *
     * イベント発火のチェック。
     * 入力する時間は呼ばれるモーションタイミングを０とした秒数で行う。
     *
     * @param beforeCheckTimeSeconds 前回のイベントチェック時間[秒]
     * @param motionTimeSeconds 今回の再生時間[秒]
     */
    getFiredEvent(beforeCheckTimeSeconds, motionTimeSeconds) {
      return this._firedEventValues;
    }
    /**
     * 透明度のカーブが存在するかどうかを確認する
     *
     * @return true  -> キーが存在する
     *          false -> キーが存在しない
     */
    isExistModelOpacity() {
      return false;
    }
    /**
     * 透明度のカーブのインデックスを返す
     *
     * @return success:透明度のカーブのインデックス
     */
    getModelOpacityIndex() {
      return -1;
    }
    /**
     * 透明度のIdを返す
     *
     * @param index モーションカーブのインデックス
     * @return success:透明度のId
     */
    getModelOpacityId(index) {
      return null;
    }
    /**
     * 指定時間の透明度の値を返す
     *
     * @return success:モーションの現在時間におけるOpacityの値
     *
     * @note  更新後の値を取るにはUpdateParameters() の後に呼び出す。
     */
    getModelOpacityValue() {
      return 1;
    }
    /**
     * 終了時刻の調整
     * @param motionQueueEntry CubismMotionQueueManagerで管理されているモーション
     */
    adjustEndTime(motionQueueEntry) {
      const duration = this.getDuration();
      const endTime = duration <= 0 ? -1 : motionQueueEntry.getStartTime() + duration;
      motionQueueEntry.setEndTime(endTime);
    }
  };
  var Live2DCubismFramework21;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.ACubismMotion = ACubismMotion;
  })(Live2DCubismFramework21 || (Live2DCubismFramework21 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismexpressionmotion.ts
  var ExpressionKeyFadeIn = "FadeInTime";
  var ExpressionKeyFadeOut = "FadeOutTime";
  var ExpressionKeyParameters = "Parameters";
  var ExpressionKeyId = "Id";
  var ExpressionKeyValue = "Value";
  var ExpressionKeyBlend = "Blend";
  var BlendValueAdd = "Add";
  var BlendValueMultiply = "Multiply";
  var BlendValueOverwrite = "Overwrite";
  var DefaultFadeTime = 1;
  var _CubismExpressionMotion = class _CubismExpressionMotion extends ACubismMotion {
    // 乗算適用の初期値
    /**
     * インスタンスを作成する。
     * @param buffer expファイルが読み込まれているバッファ
     * @param size バッファのサイズ
     * @return 作成されたインスタンス
     */
    static create(buffer, size) {
      const expression = new _CubismExpressionMotion();
      expression.parse(buffer, size);
      return expression;
    }
    /**
     * モデルのパラメータの更新の実行
     * @param model 対象のモデル
     * @param userTimeSeconds デルタ時間の積算値[秒]
     * @param weight モーションの重み
     * @param motionQueueEntry CubismMotionQueueManagerで管理されているモーション
     */
    doUpdateParameters(model, userTimeSeconds, weight, motionQueueEntry) {
      for (let i = 0; i < this._parameters.length; ++i) {
        const parameter = this._parameters[i];
        switch (parameter.blendType) {
          case 0 /* Additive */: {
            model.addParameterValueById(
              parameter.parameterId,
              parameter.value,
              weight
            );
            break;
          }
          case 1 /* Multiply */: {
            model.multiplyParameterValueById(
              parameter.parameterId,
              parameter.value,
              weight
            );
            break;
          }
          case 2 /* Overwrite */: {
            model.setParameterValueById(
              parameter.parameterId,
              parameter.value,
              weight
            );
            break;
          }
          default:
            break;
        }
      }
    }
    /**
     * @brief 表情によるモデルのパラメータの計算
     *
     * モデルの表情に関するパラメータを計算する。
     *
     * @param[in]   model                        対象のモデル
     * @param[in]   userTimeSeconds              デルタ時間の積算値[秒]
     * @param[in]   motionQueueEntry             CubismMotionQueueManagerで管理されているモーション
     * @param[in]   expressionParameterValues    モデルに適用する各パラメータの値
     * @param[in]   expressionIndex              表情のインデックス
     * @param[in]   fadeWeight                   表情のウェイト
     */
    calculateExpressionParameters(model, userTimeSeconds, motionQueueEntry, expressionParameterValues, expressionIndex, fadeWeight) {
      if (motionQueueEntry == null || expressionParameterValues == null) {
        return;
      }
      if (!motionQueueEntry.isAvailable()) {
        return;
      }
      for (let i = 0; i < expressionParameterValues.length; ++i) {
        const expressionParameterValue = expressionParameterValues[i];
        if (expressionParameterValue.parameterId == null) {
          continue;
        }
        const currentParameterValue = expressionParameterValue.overwriteValue = model.getParameterValueById(expressionParameterValue.parameterId);
        const expressionParameters = this.getExpressionParameters();
        let parameterIndex = -1;
        for (let j = 0; j < expressionParameters.length; ++j) {
          if (expressionParameterValue.parameterId != expressionParameters[j].parameterId) {
            continue;
          }
          parameterIndex = j;
          break;
        }
        if (parameterIndex < 0) {
          if (expressionIndex == 0) {
            expressionParameterValue.additiveValue = _CubismExpressionMotion.DefaultAdditiveValue;
            expressionParameterValue.multiplyValue = _CubismExpressionMotion.DefaultMultiplyValue;
            expressionParameterValue.overwriteValue = currentParameterValue;
          } else {
            expressionParameterValue.additiveValue = this.calculateValue(
              expressionParameterValue.additiveValue,
              _CubismExpressionMotion.DefaultAdditiveValue,
              fadeWeight
            );
            expressionParameterValue.multiplyValue = this.calculateValue(
              expressionParameterValue.multiplyValue,
              _CubismExpressionMotion.DefaultMultiplyValue,
              fadeWeight
            );
            expressionParameterValue.overwriteValue = this.calculateValue(
              expressionParameterValue.overwriteValue,
              currentParameterValue,
              fadeWeight
            );
          }
          continue;
        }
        const value = expressionParameters[parameterIndex].value;
        let newAdditiveValue, newMultiplyValue, newOverwriteValue;
        switch (expressionParameters[parameterIndex].blendType) {
          case 0 /* Additive */:
            newAdditiveValue = value;
            newMultiplyValue = _CubismExpressionMotion.DefaultMultiplyValue;
            newOverwriteValue = currentParameterValue;
            break;
          case 1 /* Multiply */:
            newAdditiveValue = _CubismExpressionMotion.DefaultAdditiveValue;
            newMultiplyValue = value;
            newOverwriteValue = currentParameterValue;
            break;
          case 2 /* Overwrite */:
            newAdditiveValue = _CubismExpressionMotion.DefaultAdditiveValue;
            newMultiplyValue = _CubismExpressionMotion.DefaultMultiplyValue;
            newOverwriteValue = value;
            break;
          default:
            return;
        }
        if (expressionIndex == 0) {
          expressionParameterValue.additiveValue = newAdditiveValue;
          expressionParameterValue.multiplyValue = newMultiplyValue;
          expressionParameterValue.overwriteValue = newOverwriteValue;
        } else {
          expressionParameterValue.additiveValue = expressionParameterValue.additiveValue * (1 - fadeWeight) + newAdditiveValue * fadeWeight;
          expressionParameterValue.multiplyValue = expressionParameterValue.multiplyValue * (1 - fadeWeight) + newMultiplyValue * fadeWeight;
          expressionParameterValue.overwriteValue = expressionParameterValue.overwriteValue * (1 - fadeWeight) + newOverwriteValue * fadeWeight;
        }
      }
    }
    /**
     * @brief 表情が参照しているパラメータを取得
     *
     * 表情が参照しているパラメータを取得する
     *
     * @return 表情パラメータ
     */
    getExpressionParameters() {
      return this._parameters;
    }
    parse(buffer, size) {
      const json = CubismJson.create(buffer, size);
      if (!json) {
        return;
      }
      const root = json.getRoot();
      this.setFadeInTime(
        root.getValueByString(ExpressionKeyFadeIn).toFloat(DefaultFadeTime)
      );
      this.setFadeOutTime(
        root.getValueByString(ExpressionKeyFadeOut).toFloat(DefaultFadeTime)
      );
      const parameterCount = root.getValueByString(ExpressionKeyParameters).getSize();
      let dstIndex = this._parameters.length;
      this._parameters.length += parameterCount;
      for (let i = 0; i < parameterCount; ++i) {
        const param = root.getValueByString(ExpressionKeyParameters).getValueByIndex(i);
        const parameterId = CubismFramework.getIdManager().getId(
          param.getValueByString(ExpressionKeyId).getRawString()
        );
        const value = param.getValueByString(ExpressionKeyValue).toFloat();
        let blendType;
        if (param.getValueByString(ExpressionKeyBlend).isNull() || param.getValueByString(ExpressionKeyBlend).getString() == BlendValueAdd) {
          blendType = 0 /* Additive */;
        } else if (param.getValueByString(ExpressionKeyBlend).getString() == BlendValueMultiply) {
          blendType = 1 /* Multiply */;
        } else if (param.getValueByString(ExpressionKeyBlend).getString() == BlendValueOverwrite) {
          blendType = 2 /* Overwrite */;
        } else {
          blendType = 0 /* Additive */;
        }
        const item = new ExpressionParameter();
        item.parameterId = parameterId;
        item.blendType = blendType;
        item.value = value;
        this._parameters[dstIndex++] = item;
      }
      CubismJson.delete(json);
    }
    /**
     * @brief ブレンド計算
     *
     * 入力された値でブレンド計算をする。
     *
     * @param source 現在の値
     * @param destination 適用する値
     * @param weight ウェイト
     * @return 計算結果
     */
    calculateValue(source, destination, fadeWeight) {
      return source * (1 - fadeWeight) + destination * fadeWeight;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._parameters = new Array();
    }
    // 表情のパラメータ情報リスト
  };
  _CubismExpressionMotion.DefaultAdditiveValue = 0;
  // 加算適用の初期値
  _CubismExpressionMotion.DefaultMultiplyValue = 1;
  var CubismExpressionMotion = _CubismExpressionMotion;
  var ExpressionBlendType = /* @__PURE__ */ ((ExpressionBlendType2) => {
    ExpressionBlendType2[ExpressionBlendType2["Additive"] = 0] = "Additive";
    ExpressionBlendType2[ExpressionBlendType2["Multiply"] = 1] = "Multiply";
    ExpressionBlendType2[ExpressionBlendType2["Overwrite"] = 2] = "Overwrite";
    return ExpressionBlendType2;
  })(ExpressionBlendType || {});
  var ExpressionParameter = class {
    // 値
  };
  var Live2DCubismFramework22;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismExpressionMotion = CubismExpressionMotion;
    Live2DCubismFramework51.ExpressionBlendType = ExpressionBlendType;
    Live2DCubismFramework51.ExpressionParameter = ExpressionParameter;
  })(Live2DCubismFramework22 || (Live2DCubismFramework22 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismmotionqueueentry.ts
  var CubismMotionQueueEntry = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._autoDelete = false;
      this._motion = null;
      this._available = true;
      this._finished = false;
      this._started = false;
      this._startTimeSeconds = -1;
      this._fadeInStartTimeSeconds = 0;
      this._endTimeSeconds = -1;
      this._stateTimeSeconds = 0;
      this._stateWeight = 0;
      this._lastEventCheckSeconds = 0;
      this._motionQueueEntryHandle = this;
      this._fadeOutSeconds = 0;
      this._isTriggeredFadeOut = false;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      if (this._autoDelete && this._motion) {
        ACubismMotion.delete(this._motion);
      }
    }
    /**
     * フェードアウト時間と開始判定の設定
     * @param fadeOutSeconds フェードアウトにかかる時間[秒]
     */
    setFadeOut(fadeOutSeconds) {
      this._fadeOutSeconds = fadeOutSeconds;
      this._isTriggeredFadeOut = true;
    }
    /**
     * フェードアウトの開始
     * @param fadeOutSeconds フェードアウトにかかる時間[秒]
     * @param userTimeSeconds デルタ時間の積算値[秒]
     */
    startFadeOut(fadeOutSeconds, userTimeSeconds) {
      const newEndTimeSeconds = userTimeSeconds + fadeOutSeconds;
      this._isTriggeredFadeOut = true;
      if (this._endTimeSeconds < 0 || newEndTimeSeconds < this._endTimeSeconds) {
        this._endTimeSeconds = newEndTimeSeconds;
      }
    }
    /**
     * モーションの終了の確認
     *
     * @return true モーションが終了した
     * @return false 終了していない
     */
    isFinished() {
      return this._finished;
    }
    /**
     * モーションの開始の確認
     * @return true モーションが開始した
     * @return false 開始していない
     */
    isStarted() {
      return this._started;
    }
    /**
     * モーションの開始時刻の取得
     * @return モーションの開始時刻[秒]
     */
    getStartTime() {
      return this._startTimeSeconds;
    }
    /**
     * フェードインの開始時刻の取得
     * @return フェードインの開始時刻[秒]
     */
    getFadeInStartTime() {
      return this._fadeInStartTimeSeconds;
    }
    /**
     * フェードインの終了時刻の取得
     * @return フェードインの終了時刻の取得
     */
    getEndTime() {
      return this._endTimeSeconds;
    }
    /**
     * モーションの開始時刻の設定
     * @param startTime モーションの開始時刻
     */
    setStartTime(startTime) {
      this._startTimeSeconds = startTime;
    }
    /**
     * フェードインの開始時刻の設定
     * @param startTime フェードインの開始時刻[秒]
     */
    setFadeInStartTime(startTime) {
      this._fadeInStartTimeSeconds = startTime;
    }
    /**
     * フェードインの終了時刻の設定
     * @param endTime フェードインの終了時刻[秒]
     */
    setEndTime(endTime) {
      this._endTimeSeconds = endTime;
    }
    /**
     * モーションの終了の設定
     * @param f trueならモーションの終了
     */
    setIsFinished(f) {
      this._finished = f;
    }
    /**
     * モーション開始の設定
     * @param f trueならモーションの開始
     */
    setIsStarted(f) {
      this._started = f;
    }
    /**
     * モーションの有効性の確認
     * @return true モーションは有効
     * @return false モーションは無効
     */
    isAvailable() {
      return this._available;
    }
    /**
     * モーションの有効性の設定
     * @param v trueならモーションは有効
     */
    setIsAvailable(v) {
      this._available = v;
    }
    /**
     * モーションの状態の設定
     * @param timeSeconds 現在時刻[秒]
     * @param weight モーション尾重み
     */
    setState(timeSeconds, weight) {
      this._stateTimeSeconds = timeSeconds;
      this._stateWeight = weight;
    }
    /**
     * モーションの現在時刻の取得
     * @return モーションの現在時刻[秒]
     */
    getStateTime() {
      return this._stateTimeSeconds;
    }
    /**
     * モーションの重みの取得
     * @return モーションの重み
     */
    getStateWeight() {
      return this._stateWeight;
    }
    /**
     * 最後にイベントの発火をチェックした時間を取得
     *
     * @return 最後にイベントの発火をチェックした時間[秒]
     */
    getLastCheckEventSeconds() {
      return this._lastEventCheckSeconds;
    }
    /**
     * 最後にイベントをチェックした時間を設定
     * @param checkSeconds 最後にイベントをチェックした時間[秒]
     */
    setLastCheckEventSeconds(checkSeconds) {
      this._lastEventCheckSeconds = checkSeconds;
    }
    /**
     * フェードアウト開始判定の取得
     * @return フェードアウト開始するかどうか
     */
    isTriggeredFadeOut() {
      return this._isTriggeredFadeOut;
    }
    /**
     * フェードアウト時間の取得
     * @return フェードアウト時間[秒]
     */
    getFadeOutSeconds() {
      return this._fadeOutSeconds;
    }
    /**
     * モーションの取得
     *
     * @return モーション
     */
    getCubismMotion() {
      return this._motion;
    }
    // インスタンスごとに一意の値を持つ識別番号
  };
  var Live2DCubismFramework23;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMotionQueueEntry = CubismMotionQueueEntry;
  })(Live2DCubismFramework23 || (Live2DCubismFramework23 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismmotionqueuemanager.ts
  var CubismMotionQueueManager = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._userTimeSeconds = 0;
      this._eventCallBack = null;
      this._eventCustomData = null;
      this._motions = new Array();
    }
    /**
     * デストラクタ
     */
    release() {
      for (let i = 0; i < this._motions.length; ++i) {
        if (this._motions[i]) {
          this._motions[i].release();
          this._motions[i] = null;
        }
      }
      this._motions = null;
    }
    /**
     * 指定したモーションの開始
     *
     * 指定したモーションを開始する。同じタイプのモーションが既にある場合は、既存のモーションに終了フラグを立て、フェードアウトを開始させる。
     *
     * @param   motion          開始するモーション
     * @param   autoDelete      再生が終了したモーションのインスタンスを削除するなら true
     * @param   userTimeSeconds Deprecated: デルタ時間の積算値[秒] 関数内で参照していないため使用は非推奨。
     * @return                      開始したモーションの識別番号を返す。個別のモーションが終了したか否かを判定するIsFinished()の引数で使用する。開始できない時は「-1」
     */
    startMotion(motion, autoDelete, userTimeSeconds) {
      if (motion == null) {
        return InvalidMotionQueueEntryHandleValue;
      }
      let motionQueueEntry = null;
      for (let i = 0; i < this._motions.length; ++i) {
        motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          continue;
        }
        motionQueueEntry.setFadeOut(motionQueueEntry._motion.getFadeOutTime());
      }
      motionQueueEntry = new CubismMotionQueueEntry();
      motionQueueEntry._autoDelete = autoDelete;
      motionQueueEntry._motion = motion;
      this._motions.push(motionQueueEntry);
      return motionQueueEntry._motionQueueEntryHandle;
    }
    /**
     * 全てのモーションの終了の確認
     * @return true 全て終了している
     * @return false 終了していない
     */
    isFinished() {
      for (let i = 0; i < this._motions.length; ) {
        let motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          this._motions.splice(i, 1);
          continue;
        }
        const motion = motionQueueEntry._motion;
        if (motion == null) {
          motionQueueEntry.release();
          motionQueueEntry = null;
          this._motions.splice(i, 1);
          continue;
        }
        if (!motionQueueEntry.isFinished()) {
          return false;
        } else {
          i++;
        }
      }
      return true;
    }
    /**
     * 指定したモーションの終了の確認
     * @param motionQueueEntryNumber モーションの識別番号
     * @return true 全て終了している
     * @return false 終了していない
     */
    isFinishedByHandle(motionQueueEntryNumber) {
      for (let i = 0; i < this._motions.length; i++) {
        const motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          continue;
        }
        if (motionQueueEntry._motionQueueEntryHandle == motionQueueEntryNumber && !motionQueueEntry.isFinished()) {
          return false;
        }
      }
      return true;
    }
    /**
     * 全てのモーションを停止する
     */
    stopAllMotions() {
      for (let i = 0; i < this._motions.length; i++) {
        const motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          this._motions.splice(i, 1);
          continue;
        }
        motionQueueEntry.release();
        this._motions.splice(i, 1);
        continue;
      }
    }
    /**
     * @brief CubismMotionQueueEntryの配列の取得
     *
     * CubismMotionQueueEntryの配列を取得する。
     *
     * @return  CubismMotionQueueEntryの配列へのポインタ
     *          NULL   見つからなかった
     */
    getCubismMotionQueueEntries() {
      return this._motions;
    }
    /**
       * 指定したCubismMotionQueueEntryの取得
    
       * @param   motionQueueEntryNumber  モーションの識別番号
       * @return  指定したCubismMotionQueueEntry
       * @return  null   見つからなかった
       */
    getCubismMotionQueueEntry(motionQueueEntryNumber) {
      for (let i = 0; i < this._motions.length; i++) {
        const motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          continue;
        }
        if (motionQueueEntry._motionQueueEntryHandle == motionQueueEntryNumber) {
          return motionQueueEntry;
        }
      }
      return null;
    }
    /**
     * イベントを受け取るCallbackの登録
     *
     * @param callback コールバック関数
     * @param customData コールバックに返されるデータ
     */
    setEventCallback(callback, customData = null) {
      this._eventCallBack = callback;
      this._eventCustomData = customData;
    }
    /**
     * モーションを更新して、モデルにパラメータ値を反映する。
     *
     * @param   model   対象のモデル
     * @param   userTimeSeconds   デルタ時間の積算値[秒]
     * @return  true    モデルへパラメータ値の反映あり
     * @return  false   モデルへパラメータ値の反映なし(モーションの変化なし)
     */
    doUpdateMotion(model, userTimeSeconds) {
      let updated = false;
      for (let i = 0; i < this._motions.length; ) {
        let motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          this._motions.splice(i, 1);
          continue;
        }
        const motion = motionQueueEntry._motion;
        if (motion == null) {
          motionQueueEntry.release();
          motionQueueEntry = null;
          this._motions.splice(i, 1);
          continue;
        }
        motion.updateParameters(model, motionQueueEntry, userTimeSeconds);
        updated = true;
        const firedList = motion.getFiredEvent(
          motionQueueEntry.getLastCheckEventSeconds() - motionQueueEntry.getStartTime(),
          userTimeSeconds - motionQueueEntry.getStartTime()
        );
        for (let i2 = 0; i2 < firedList.length; ++i2) {
          this._eventCallBack(this, firedList[i2], this._eventCustomData);
        }
        motionQueueEntry.setLastCheckEventSeconds(userTimeSeconds);
        if (motionQueueEntry.isFinished()) {
          motionQueueEntry.release();
          motionQueueEntry = null;
          this._motions.splice(i, 1);
        } else {
          if (motionQueueEntry.isTriggeredFadeOut()) {
            motionQueueEntry.startFadeOut(
              motionQueueEntry.getFadeOutSeconds(),
              userTimeSeconds
            );
          }
          i++;
        }
      }
      return updated;
    }
    // コールバックに戻されるデータ
  };
  var InvalidMotionQueueEntryHandleValue = -1;
  var Live2DCubismFramework24;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMotionQueueManager = CubismMotionQueueManager;
    Live2DCubismFramework51.InvalidMotionQueueEntryHandleValue = InvalidMotionQueueEntryHandleValue;
  })(Live2DCubismFramework24 || (Live2DCubismFramework24 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismexpressionmotionmanager.ts
  var ExpressionParameterValue = class {
    // 上書き値
  };
  var CubismExpressionMotionManager = class extends CubismMotionQueueManager {
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._expressionParameterValues = new Array();
      this._fadeWeights = new Array();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      if (this._expressionParameterValues) {
        csmDelete(this._expressionParameterValues);
        this._expressionParameterValues = null;
      }
      if (this._fadeWeights) {
        csmDelete(this._fadeWeights);
        this._fadeWeights = null;
      }
    }
    /**
     * @brief 再生中のモーションのウェイトを取得する。
     *
     * @param[in]    index    表情のインデックス
     * @return               表情モーションのウェイト
     */
    getFadeWeight(index) {
      if (index < 0 || this._fadeWeights.length < 1 || index >= this._fadeWeights.length) {
        console.warn(
          "Failed to get the fade weight value. The element at that index does not exist."
        );
        return -1;
      }
      return this._fadeWeights[index];
    }
    /**
     * @brief モーションのウェイトの設定。
     *
     * @param[in]    index    表情のインデックス
     * @param[in]    index    表情モーションのウェイト
     */
    setFadeWeight(index, expressionFadeWeight) {
      if (index < 0 || this._fadeWeights.length < 1 || this._fadeWeights.length <= index) {
        console.warn(
          "Failed to set the fade weight value. The element at that index does not exist."
        );
        return;
      }
      this._fadeWeights[index] = expressionFadeWeight;
    }
    /**
     * @brief モーションの更新
     *
     * モーションを更新して、モデルにパラメータ値を反映する。
     *
     * @param[in]   model   対象のモデル
     * @param[in]   deltaTimeSeconds    デルタ時間[秒]
     * @return  true    更新されている
     *          false   更新されていない
     */
    updateMotion(model, deltaTimeSeconds) {
      this._userTimeSeconds += deltaTimeSeconds;
      let updated = false;
      const motions = this.getCubismMotionQueueEntries();
      let expressionWeight = 0;
      let expressionIndex = 0;
      if (this._fadeWeights.length !== motions.length) {
        const difference = motions.length - this._fadeWeights.length;
        let dstIndex = this._fadeWeights.length;
        this._fadeWeights.length += difference;
        for (let i = 0; i < difference; i++) {
          this._fadeWeights[dstIndex++] = 0;
        }
      }
      for (let i = 0; i < this._motions.length; ) {
        const motionQueueEntry = this._motions[i];
        if (motionQueueEntry == null) {
          motions.splice(i, 1);
          continue;
        }
        const expressionMotion = motionQueueEntry.getCubismMotion();
        if (expressionMotion == null) {
          csmDelete(motionQueueEntry);
          motions.splice(i, 1);
          continue;
        }
        const expressionParameters = expressionMotion.getExpressionParameters();
        if (motionQueueEntry.isAvailable()) {
          for (let i2 = 0; i2 < expressionParameters.length; ++i2) {
            if (expressionParameters[i2].parameterId == null) {
              continue;
            }
            let index = -1;
            for (let j = 0; j < this._expressionParameterValues.length; ++j) {
              if (this._expressionParameterValues[j].parameterId != expressionParameters[i2].parameterId) {
                continue;
              }
              index = j;
              break;
            }
            if (index >= 0) {
              continue;
            }
            const item = new ExpressionParameterValue();
            item.parameterId = expressionParameters[i2].parameterId;
            item.additiveValue = CubismExpressionMotion.DefaultAdditiveValue;
            item.multiplyValue = CubismExpressionMotion.DefaultMultiplyValue;
            item.overwriteValue = model.getParameterValueById(item.parameterId);
            this._expressionParameterValues.push(item);
          }
        }
        expressionMotion.setupMotionQueueEntry(
          motionQueueEntry,
          this._userTimeSeconds
        );
        this.setFadeWeight(
          expressionIndex,
          expressionMotion.updateFadeWeight(
            motionQueueEntry,
            this._userTimeSeconds
          )
        );
        expressionMotion.calculateExpressionParameters(
          model,
          this._userTimeSeconds,
          motionQueueEntry,
          this._expressionParameterValues,
          expressionIndex,
          this.getFadeWeight(expressionIndex)
        );
        expressionWeight += expressionMotion.getFadeInTime() == 0 ? 1 : CubismMath.getEasingSine(
          (this._userTimeSeconds - motionQueueEntry.getFadeInStartTime()) / expressionMotion.getFadeInTime()
        );
        updated = true;
        if (motionQueueEntry.isTriggeredFadeOut()) {
          motionQueueEntry.startFadeOut(
            motionQueueEntry.getFadeOutSeconds(),
            this._userTimeSeconds
          );
        }
        ++i;
        ++expressionIndex;
      }
      if (motions.length > 1) {
        const latestFadeWeight = this.getFadeWeight(
          this._fadeWeights.length - 1
        );
        if (latestFadeWeight >= 1) {
          for (let i = motions.length - 2; i >= 0; --i) {
            const motionQueueEntry = motions[i];
            csmDelete(motionQueueEntry);
            motions.splice(i, 1);
            this._fadeWeights.splice(i, 1);
          }
        }
      }
      if (expressionWeight > 1) {
        expressionWeight = 1;
      }
      for (let i = 0; i < this._expressionParameterValues.length; ++i) {
        const expressionParameterValue = this._expressionParameterValues[i];
        model.setParameterValueById(
          expressionParameterValue.parameterId,
          (expressionParameterValue.overwriteValue + expressionParameterValue.additiveValue) * expressionParameterValue.multiplyValue,
          expressionWeight
        );
        expressionParameterValue.additiveValue = CubismExpressionMotion.DefaultAdditiveValue;
        expressionParameterValue.multiplyValue = CubismExpressionMotion.DefaultMultiplyValue;
      }
      return updated;
    }
    ///< 表情の再生開始時刻
  };
  var Live2DCubismFramework25;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismExpressionMotionManager = CubismExpressionMotionManager;
  })(Live2DCubismFramework25 || (Live2DCubismFramework25 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismmotioninternal.ts
  var CubismMotionCurveTarget = /* @__PURE__ */ ((CubismMotionCurveTarget2) => {
    CubismMotionCurveTarget2[CubismMotionCurveTarget2["CubismMotionCurveTarget_Model"] = 0] = "CubismMotionCurveTarget_Model";
    CubismMotionCurveTarget2[CubismMotionCurveTarget2["CubismMotionCurveTarget_Parameter"] = 1] = "CubismMotionCurveTarget_Parameter";
    CubismMotionCurveTarget2[CubismMotionCurveTarget2["CubismMotionCurveTarget_PartOpacity"] = 2] = "CubismMotionCurveTarget_PartOpacity";
    return CubismMotionCurveTarget2;
  })(CubismMotionCurveTarget || {});
  var CubismMotionSegmentType = /* @__PURE__ */ ((CubismMotionSegmentType2) => {
    CubismMotionSegmentType2[CubismMotionSegmentType2["CubismMotionSegmentType_Linear"] = 0] = "CubismMotionSegmentType_Linear";
    CubismMotionSegmentType2[CubismMotionSegmentType2["CubismMotionSegmentType_Bezier"] = 1] = "CubismMotionSegmentType_Bezier";
    CubismMotionSegmentType2[CubismMotionSegmentType2["CubismMotionSegmentType_Stepped"] = 2] = "CubismMotionSegmentType_Stepped";
    CubismMotionSegmentType2[CubismMotionSegmentType2["CubismMotionSegmentType_InverseStepped"] = 3] = "CubismMotionSegmentType_InverseStepped";
    return CubismMotionSegmentType2;
  })(CubismMotionSegmentType || {});
  var CubismMotionPoint = class {
    constructor() {
      this.time = 0;
      // 時間[秒]
      this.value = 0;
    }
    // 値
  };
  var CubismMotionSegment = class {
    /**
     * @brief コンストラクタ
     *
     * コンストラクタ。
     */
    constructor() {
      this.evaluate = null;
      this.basePointIndex = 0;
      this.segmentType = 0;
    }
    // セグメントの種類
  };
  var CubismMotionCurve = class {
    constructor() {
      this.type = 0 /* CubismMotionCurveTarget_Model */;
      this.segmentCount = 0;
      this.baseSegmentIndex = 0;
      this.fadeInTime = 0;
      this.fadeOutTime = 0;
    }
    // フェードアウトにかかる時間[秒]
  };
  var CubismMotionEvent = class {
    constructor() {
      this.fireTime = 0;
    }
  };
  var CubismMotionData = class {
    constructor() {
      this.duration = 0;
      this.loop = false;
      this.curveCount = 0;
      this.eventCount = 0;
      this.fps = 0;
      this.curves = new Array();
      this.segments = new Array();
      this.points = new Array();
      this.events = new Array();
    }
    // イベントのリスト
  };
  var Live2DCubismFramework26;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMotionCurve = CubismMotionCurve;
    Live2DCubismFramework51.CubismMotionCurveTarget = CubismMotionCurveTarget;
    Live2DCubismFramework51.CubismMotionData = CubismMotionData;
    Live2DCubismFramework51.CubismMotionEvent = CubismMotionEvent;
    Live2DCubismFramework51.CubismMotionPoint = CubismMotionPoint;
    Live2DCubismFramework51.CubismMotionSegment = CubismMotionSegment;
    Live2DCubismFramework51.CubismMotionSegmentType = CubismMotionSegmentType;
  })(Live2DCubismFramework26 || (Live2DCubismFramework26 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismmotionjson.ts
  var Meta = "Meta";
  var Duration = "Duration";
  var Loop = "Loop";
  var AreBeziersRestricted = "AreBeziersRestricted";
  var CurveCount = "CurveCount";
  var Fps = "Fps";
  var TotalSegmentCount = "TotalSegmentCount";
  var TotalPointCount = "TotalPointCount";
  var Curves = "Curves";
  var Target = "Target";
  var Id2 = "Id";
  var FadeInTime = "FadeInTime";
  var FadeOutTime = "FadeOutTime";
  var Segments = "Segments";
  var UserData = "UserData";
  var UserDataCount = "UserDataCount";
  var TotalUserDataSize = "TotalUserDataSize";
  var Time = "Time";
  var Value6 = "Value";
  var CubismMotionJson = class {
    /**
     * コンストラクタ
     * @param buffer motion3.jsonが読み込まれているバッファ
     * @param size バッファのサイズ
     */
    constructor(buffer, size) {
      this._json = CubismJson.create(buffer, size);
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      CubismJson.delete(this._json);
    }
    /**
     * モーションの長さを取得する
     * @return モーションの長さ[秒]
     */
    getMotionDuration() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(Duration).toFloat();
    }
    /**
     * モーションのループ情報の取得
     * @return true ループする
     * @return false ループしない
     */
    isMotionLoop() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(Loop).toBoolean();
    }
    /**
     *  motion3.jsonファイルの整合性チェック
     *
     * @return 正常なファイルの場合はtrueを返す。
     */
    hasConsistency() {
      let result = true;
      if (!this._json || !this._json.getRoot()) {
        return false;
      }
      const actualCurveListSize = this._json.getRoot().getValueByString(Curves).getVector().length;
      let actualTotalSegmentCount = 0;
      let actualTotalPointCount = 0;
      for (let curvePosition = 0; curvePosition < actualCurveListSize; ++curvePosition) {
        for (let segmentPosition = 0; segmentPosition < this.getMotionCurveSegmentCount(curvePosition); ) {
          if (segmentPosition == 0) {
            actualTotalPointCount += 1;
            segmentPosition += 2;
          }
          const segment = this.getMotionCurveSegment(
            curvePosition,
            segmentPosition
          );
          switch (segment) {
            case 0 /* CubismMotionSegmentType_Linear */:
              actualTotalPointCount += 1;
              segmentPosition += 3;
              break;
            case 1 /* CubismMotionSegmentType_Bezier */:
              actualTotalPointCount += 3;
              segmentPosition += 7;
              break;
            case 2 /* CubismMotionSegmentType_Stepped */:
              actualTotalPointCount += 1;
              segmentPosition += 3;
              break;
            case 3 /* CubismMotionSegmentType_InverseStepped */:
              actualTotalPointCount += 1;
              segmentPosition += 3;
              break;
            default:
              CSM_ASSERT(0);
              break;
          }
          ++actualTotalSegmentCount;
        }
      }
      if (actualCurveListSize != this.getMotionCurveCount()) {
        CubismLogWarning("The number of curves does not match the metadata.");
        result = false;
      }
      if (actualTotalSegmentCount != this.getMotionTotalSegmentCount()) {
        CubismLogWarning("The number of segment does not match the metadata.");
        result = false;
      }
      if (actualTotalPointCount != this.getMotionTotalPointCount()) {
        CubismLogWarning("The number of point does not match the metadata.");
        result = false;
      }
      return result;
    }
    getEvaluationOptionFlag(flagType) {
      if (0 /* EvaluationOptionFlag_AreBeziersRistricted */ == flagType) {
        return this._json.getRoot().getValueByString(Meta).getValueByString(AreBeziersRestricted).toBoolean();
      }
      return false;
    }
    /**
     * モーションカーブの個数の取得
     * @return モーションカーブの個数
     */
    getMotionCurveCount() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(CurveCount).toInt();
    }
    /**
     * モーションのフレームレートの取得
     * @return フレームレート[FPS]
     */
    getMotionFps() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(Fps).toFloat();
    }
    /**
     * モーションのセグメントの総合計の取得
     * @return モーションのセグメントの取得
     */
    getMotionTotalSegmentCount() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(TotalSegmentCount).toInt();
    }
    /**
     * モーションのカーブの制御店の総合計の取得
     * @return モーションのカーブの制御点の総合計
     */
    getMotionTotalPointCount() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(TotalPointCount).toInt();
    }
    /**
     * モーションのフェードイン時間の存在
     * @return true 存在する
     * @return false 存在しない
     */
    isExistMotionFadeInTime() {
      return !this._json.getRoot().getValueByString(Meta).getValueByString(FadeInTime).isNull();
    }
    /**
     * モーションのフェードアウト時間の存在
     * @return true 存在する
     * @return false 存在しない
     */
    isExistMotionFadeOutTime() {
      return !this._json.getRoot().getValueByString(Meta).getValueByString(FadeOutTime).isNull();
    }
    /**
     * モーションのフェードイン時間の取得
     * @return フェードイン時間[秒]
     */
    getMotionFadeInTime() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(FadeInTime).toFloat();
    }
    /**
     * モーションのフェードアウト時間の取得
     * @return フェードアウト時間[秒]
     */
    getMotionFadeOutTime() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(FadeOutTime).toFloat();
    }
    /**
     * モーションのカーブの種類の取得
     * @param curveIndex カーブのインデックス
     * @return カーブの種類
     */
    getMotionCurveTarget(curveIndex) {
      return this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(Target).getRawString();
    }
    /**
     * モーションのカーブのIDの取得
     * @param curveIndex カーブのインデックス
     * @return カーブのID
     */
    getMotionCurveId(curveIndex) {
      return CubismFramework.getIdManager().getId(
        this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(Id2).getRawString()
      );
    }
    /**
     * モーションのカーブのフェードイン時間の存在
     * @param curveIndex カーブのインデックス
     * @return true 存在する
     * @return false 存在しない
     */
    isExistMotionCurveFadeInTime(curveIndex) {
      return !this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(FadeInTime).isNull();
    }
    /**
     * モーションのカーブのフェードアウト時間の存在
     * @param curveIndex カーブのインデックス
     * @return true 存在する
     * @return false 存在しない
     */
    isExistMotionCurveFadeOutTime(curveIndex) {
      return !this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(FadeOutTime).isNull();
    }
    /**
     * モーションのカーブのフェードイン時間の取得
     * @param curveIndex カーブのインデックス
     * @return フェードイン時間[秒]
     */
    getMotionCurveFadeInTime(curveIndex) {
      return this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(FadeInTime).toFloat();
    }
    /**
     * モーションのカーブのフェードアウト時間の取得
     * @param curveIndex カーブのインデックス
     * @return フェードアウト時間[秒]
     */
    getMotionCurveFadeOutTime(curveIndex) {
      return this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(FadeOutTime).toFloat();
    }
    /**
     * モーションのカーブのセグメントの個数を取得する
     * @param curveIndex カーブのインデックス
     * @return モーションのカーブのセグメントの個数
     */
    getMotionCurveSegmentCount(curveIndex) {
      return this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(Segments).getVector().length;
    }
    /**
     * モーションのカーブのセグメントの値の取得
     * @param curveIndex カーブのインデックス
     * @param segmentIndex セグメントのインデックス
     * @return セグメントの値
     */
    getMotionCurveSegment(curveIndex, segmentIndex) {
      return this._json.getRoot().getValueByString(Curves).getValueByIndex(curveIndex).getValueByString(Segments).getValueByIndex(segmentIndex).toFloat();
    }
    /**
     * イベントの個数の取得
     * @return イベントの個数
     */
    getEventCount() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(UserDataCount).toInt();
    }
    /**
     *  イベントの総文字数の取得
     * @return イベントの総文字数
     */
    getTotalEventValueSize() {
      return this._json.getRoot().getValueByString(Meta).getValueByString(TotalUserDataSize).toInt();
    }
    /**
     * イベントの時間の取得
     * @param userDataIndex イベントのインデックス
     * @return イベントの時間[秒]
     */
    getEventTime(userDataIndex) {
      return this._json.getRoot().getValueByString(UserData).getValueByIndex(userDataIndex).getValueByString(Time).toFloat();
    }
    /**
     * イベントの取得
     * @param userDataIndex イベントのインデックス
     * @return イベントの文字列
     */
    getEventValue(userDataIndex) {
      return this._json.getRoot().getValueByString(UserData).getValueByIndex(userDataIndex).getValueByString(Value6).getRawString();
    }
    // motion3.jsonのデータ
  };
  var Live2DCubismFramework27;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMotionJson = CubismMotionJson;
  })(Live2DCubismFramework27 || (Live2DCubismFramework27 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismmotion.ts
  var EffectNameEyeBlink = "EyeBlink";
  var EffectNameLipSync = "LipSync";
  var TargetNameModel = "Model";
  var TargetNameParameter = "Parameter";
  var TargetNamePartOpacity = "PartOpacity";
  var IdNameOpacity = "Opacity";
  var UseOldBeziersCurveMotion = false;
  function lerpPoints(a, b, t) {
    const result = new CubismMotionPoint();
    result.time = a.time + (b.time - a.time) * t;
    result.value = a.value + (b.value - a.value) * t;
    return result;
  }
  function linearEvaluate(points, time) {
    let t = (time - points[0].time) / (points[1].time - points[0].time);
    if (t < 0) {
      t = 0;
    }
    return points[0].value + (points[1].value - points[0].value) * t;
  }
  function bezierEvaluate(points, time) {
    let t = (time - points[0].time) / (points[3].time - points[0].time);
    if (t < 0) {
      t = 0;
    }
    const p01 = lerpPoints(points[0], points[1], t);
    const p12 = lerpPoints(points[1], points[2], t);
    const p23 = lerpPoints(points[2], points[3], t);
    const p012 = lerpPoints(p01, p12, t);
    const p123 = lerpPoints(p12, p23, t);
    return lerpPoints(p012, p123, t).value;
  }
  function bezierEvaluateCardanoInterpretation(points, time) {
    const x = time;
    const x1 = points[0].time;
    const x2 = points[3].time;
    const cx1 = points[1].time;
    const cx2 = points[2].time;
    const a = x2 - 3 * cx2 + 3 * cx1 - x1;
    const b = 3 * cx2 - 6 * cx1 + 3 * x1;
    const c = 3 * cx1 - 3 * x1;
    const d = x1 - x;
    const t = CubismMath.cardanoAlgorithmForBezier(a, b, c, d);
    const p01 = lerpPoints(points[0], points[1], t);
    const p12 = lerpPoints(points[1], points[2], t);
    const p23 = lerpPoints(points[2], points[3], t);
    const p012 = lerpPoints(p01, p12, t);
    const p123 = lerpPoints(p12, p23, t);
    return lerpPoints(p012, p123, t).value;
  }
  function steppedEvaluate(points, time) {
    return points[0].value;
  }
  function inverseSteppedEvaluate(points, time) {
    return points[1].value;
  }
  function evaluateCurve(motionData, index, time, isCorrection, endTime) {
    const curve = motionData.curves[index];
    let target = -1;
    const totalSegmentCount = curve.baseSegmentIndex + curve.segmentCount;
    let pointPosition = 0;
    for (let i = curve.baseSegmentIndex; i < totalSegmentCount; ++i) {
      pointPosition = motionData.segments[i].basePointIndex + (motionData.segments[i].segmentType == 1 /* CubismMotionSegmentType_Bezier */ ? 3 : 1);
      if (motionData.points[pointPosition].time > time) {
        target = i;
        break;
      }
    }
    if (target == -1) {
      if (isCorrection && time < endTime) {
        return correctEndPoint(
          motionData,
          totalSegmentCount - 1,
          motionData.segments[curve.baseSegmentIndex].basePointIndex,
          pointPosition,
          time,
          endTime
        );
      }
      return motionData.points[pointPosition].value;
    }
    const segment = motionData.segments[target];
    return segment.evaluate(
      motionData.points.slice(segment.basePointIndex),
      time
    );
  }
  function correctEndPoint(motionData, segmentIndex, beginIndex, endIndex, time, endTime) {
    const motionPoint = [
      new CubismMotionPoint(),
      new CubismMotionPoint()
    ];
    {
      const src = motionData.points[endIndex];
      motionPoint[0].time = src.time;
      motionPoint[0].value = src.value;
    }
    {
      const src = motionData.points[beginIndex];
      motionPoint[1].time = endTime;
      motionPoint[1].value = src.value;
    }
    switch (motionData.segments[segmentIndex].segmentType) {
      case 0 /* CubismMotionSegmentType_Linear */:
      case 1 /* CubismMotionSegmentType_Bezier */:
      default:
        return linearEvaluate(motionPoint, time);
      case 2 /* CubismMotionSegmentType_Stepped */:
        return steppedEvaluate(motionPoint, time);
      case 3 /* CubismMotionSegmentType_InverseStepped */:
        return inverseSteppedEvaluate(motionPoint, time);
    }
  }
  var CubismMotion = class _CubismMotion extends ACubismMotion {
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      // mtnファイルで定義される一連のモーションの長さ
      this._motionBehavior = 1 /* MotionBehavior_V2 */;
      this._sourceFrameRate = 30;
      this._loopDurationSeconds = -1;
      this._isLoop = false;
      this._isLoopFadeIn = true;
      this._lastWeight = 0;
      this._motionData = null;
      this._modelCurveIdEyeBlink = null;
      this._modelCurveIdLipSync = null;
      this._modelCurveIdOpacity = null;
      this._eyeBlinkParameterIds = null;
      this._lipSyncParameterIds = null;
      this._modelOpacity = 1;
      this._debugMode = false;
    }
    /**
     * インスタンスを作成する
     *
     * @param buffer motion3.jsonが読み込まれているバッファ
     * @param size バッファのサイズ
     * @param onFinishedMotionHandler モーション再生終了時に呼び出されるコールバック関数
     * @param onBeganMotionHandler モーション再生開始時に呼び出されるコールバック関数
     * @param shouldCheckMotionConsistency motion3.json整合性チェックするかどうか
     * @return 作成されたインスタンス
     */
    static create(buffer, size, onFinishedMotionHandler, onBeganMotionHandler, shouldCheckMotionConsistency = false) {
      const ret = new _CubismMotion();
      ret.parse(buffer, size, shouldCheckMotionConsistency);
      if (ret._motionData) {
        ret._sourceFrameRate = ret._motionData.fps;
        ret._loopDurationSeconds = ret._motionData.duration;
        ret._onFinishedMotion = onFinishedMotionHandler;
        ret._onBeganMotion = onBeganMotionHandler;
      } else {
        csmDelete(ret);
        return null;
      }
      return ret;
    }
    /**
     * モデルのパラメータの更新の実行
     * @param model             対象のモデル
     * @param userTimeSeconds   現在の時刻[秒]
     * @param fadeWeight        モーションの重み
     * @param motionQueueEntry  CubismMotionQueueManagerで管理されているモーション
     */
    doUpdateParameters(model, userTimeSeconds, fadeWeight, motionQueueEntry) {
      if (this._modelCurveIdEyeBlink == null) {
        this._modelCurveIdEyeBlink = CubismFramework.getIdManager().getId(EffectNameEyeBlink);
      }
      if (this._modelCurveIdLipSync == null) {
        this._modelCurveIdLipSync = CubismFramework.getIdManager().getId(EffectNameLipSync);
      }
      if (this._modelCurveIdOpacity == null) {
        this._modelCurveIdOpacity = CubismFramework.getIdManager().getId(IdNameOpacity);
      }
      if (this._motionBehavior === 1 /* MotionBehavior_V2 */) {
        if (this._previousLoopState !== this._isLoop) {
          this.adjustEndTime(motionQueueEntry);
          this._previousLoopState = this._isLoop;
        }
      }
      let timeOffsetSeconds = userTimeSeconds - motionQueueEntry.getStartTime();
      if (timeOffsetSeconds < 0) {
        timeOffsetSeconds = 0;
      }
      let lipSyncValue = Number.MAX_VALUE;
      let eyeBlinkValue = Number.MAX_VALUE;
      const maxTargetSize = 64;
      let lipSyncFlags = 0;
      let eyeBlinkFlags = 0;
      if (this._eyeBlinkParameterIds.length > maxTargetSize) {
        CubismLogDebug(
          "too many eye blink targets : {0}",
          this._eyeBlinkParameterIds.length
        );
      }
      if (this._lipSyncParameterIds.length > maxTargetSize) {
        CubismLogDebug(
          "too many lip sync targets : {0}",
          this._lipSyncParameterIds.length
        );
      }
      const tmpFadeIn = this._fadeInSeconds <= 0 ? 1 : CubismMath.getEasingSine(
        (userTimeSeconds - motionQueueEntry.getFadeInStartTime()) / this._fadeInSeconds
      );
      const tmpFadeOut = this._fadeOutSeconds <= 0 || motionQueueEntry.getEndTime() < 0 ? 1 : CubismMath.getEasingSine(
        (motionQueueEntry.getEndTime() - userTimeSeconds) / this._fadeOutSeconds
      );
      let value;
      let c, parameterIndex;
      let time = timeOffsetSeconds;
      let duration = this._motionData.duration;
      const isCorrection = this._motionBehavior === 1 /* MotionBehavior_V2 */ && this._isLoop;
      if (this._isLoop) {
        if (this._motionBehavior === 1 /* MotionBehavior_V2 */) {
          duration += 1 / this._motionData.fps;
        }
        while (time > duration) {
          time -= duration;
        }
      }
      const curves = this._motionData.curves;
      for (c = 0; c < this._motionData.curveCount && curves[c].type == 0 /* CubismMotionCurveTarget_Model */; ++c) {
        value = evaluateCurve(this._motionData, c, time, isCorrection, duration);
        if (curves[c].id == this._modelCurveIdEyeBlink) {
          eyeBlinkValue = value;
        } else if (curves[c].id == this._modelCurveIdLipSync) {
          lipSyncValue = value;
        } else if (curves[c].id == this._modelCurveIdOpacity) {
          this._modelOpacity = value;
          model.setModelOapcity(this.getModelOpacityValue());
        }
      }
      let parameterMotionCurveCount = 0;
      for (; c < this._motionData.curveCount && curves[c].type == 1 /* CubismMotionCurveTarget_Parameter */; ++c) {
        parameterMotionCurveCount++;
        parameterIndex = model.getParameterIndex(curves[c].id);
        if (parameterIndex == -1) {
          continue;
        }
        const sourceValue = model.getParameterValueByIndex(parameterIndex);
        value = evaluateCurve(this._motionData, c, time, isCorrection, duration);
        if (eyeBlinkValue != Number.MAX_VALUE) {
          for (let i = 0; i < this._eyeBlinkParameterIds.length && i < maxTargetSize; ++i) {
            if (this._eyeBlinkParameterIds[i] == curves[c].id) {
              value *= eyeBlinkValue;
              eyeBlinkFlags |= 1 << i;
              break;
            }
          }
        }
        if (lipSyncValue != Number.MAX_VALUE) {
          for (let i = 0; i < this._lipSyncParameterIds.length && i < maxTargetSize; ++i) {
            if (this._lipSyncParameterIds[i] == curves[c].id) {
              value += lipSyncValue;
              lipSyncFlags |= 1 << i;
              break;
            }
          }
        }
        if (model.isRepeat(parameterIndex)) {
          value = model.getParameterRepeatValue(parameterIndex, value);
        }
        let v;
        if (curves[c].fadeInTime < 0 && curves[c].fadeOutTime < 0) {
          v = sourceValue + (value - sourceValue) * fadeWeight;
        } else {
          let fin;
          let fout;
          if (curves[c].fadeInTime < 0) {
            fin = tmpFadeIn;
          } else {
            fin = curves[c].fadeInTime == 0 ? 1 : CubismMath.getEasingSine(
              (userTimeSeconds - motionQueueEntry.getFadeInStartTime()) / curves[c].fadeInTime
            );
          }
          if (curves[c].fadeOutTime < 0) {
            fout = tmpFadeOut;
          } else {
            fout = curves[c].fadeOutTime == 0 || motionQueueEntry.getEndTime() < 0 ? 1 : CubismMath.getEasingSine(
              (motionQueueEntry.getEndTime() - userTimeSeconds) / curves[c].fadeOutTime
            );
          }
          const paramWeight = this._weight * fin * fout;
          v = sourceValue + (value - sourceValue) * paramWeight;
        }
        model.setParameterValueByIndex(parameterIndex, v, 1);
      }
      {
        if (eyeBlinkValue != Number.MAX_VALUE) {
          for (let i = 0; i < this._eyeBlinkParameterIds.length && i < maxTargetSize; ++i) {
            const sourceValue = model.getParameterValueById(
              this._eyeBlinkParameterIds[i]
            );
            if (eyeBlinkFlags >> i & 1) {
              continue;
            }
            const v = sourceValue + (eyeBlinkValue - sourceValue) * fadeWeight;
            model.setParameterValueById(this._eyeBlinkParameterIds[i], v);
          }
        }
        if (lipSyncValue != Number.MAX_VALUE) {
          for (let i = 0; i < this._lipSyncParameterIds.length && i < maxTargetSize; ++i) {
            const sourceValue = model.getParameterValueById(
              this._lipSyncParameterIds[i]
            );
            if (lipSyncFlags >> i & 1) {
              continue;
            }
            const v = sourceValue + (lipSyncValue - sourceValue) * fadeWeight;
            model.setParameterValueById(this._lipSyncParameterIds[i], v);
          }
        }
      }
      for (; c < this._motionData.curveCount && curves[c].type == 2 /* CubismMotionCurveTarget_PartOpacity */; ++c) {
        parameterIndex = model.getParameterIndex(curves[c].id);
        if (parameterIndex == -1) {
          continue;
        }
        value = evaluateCurve(this._motionData, c, time, isCorrection, duration);
        model.setParameterValueByIndex(parameterIndex, value);
      }
      if (timeOffsetSeconds >= duration) {
        if (this._isLoop) {
          this.updateForNextLoop(motionQueueEntry, userTimeSeconds, time);
        } else {
          if (this._onFinishedMotion) {
            this._onFinishedMotion(this);
          }
          motionQueueEntry.setIsFinished(true);
        }
      }
      this._lastWeight = fadeWeight;
    }
    /**
     * Sets the version of the Motion Behavior.
     *
     * @param Specifies the version of the Motion Behavior.
     */
    setMotionBehavior(motionBehavior) {
      this._motionBehavior = motionBehavior;
    }
    /**
     * Gets the version of the Motion Behavior.
     *
     * @return Returns the version of the Motion Behavior.
     */
    getMotionBehavior() {
      return this._motionBehavior;
    }
    /**
     * モーションの長さを取得する。
     *
     * @return  モーションの長さ[秒]
     */
    getDuration() {
      return this._isLoop ? -1 : this._loopDurationSeconds;
    }
    /**
     * モーションのループ時の長さを取得する。
     *
     * @return  モーションのループ時の長さ[秒]
     */
    getLoopDuration() {
      return this._loopDurationSeconds;
    }
    /**
     * パラメータに対するフェードインの時間を設定する。
     *
     * @param parameterId     パラメータID
     * @param value           フェードインにかかる時間[秒]
     */
    setParameterFadeInTime(parameterId, value) {
      const curves = this._motionData.curves;
      for (let i = 0; i < this._motionData.curveCount; ++i) {
        if (parameterId == curves[i].id) {
          curves[i].fadeInTime = value;
          return;
        }
      }
    }
    /**
     * パラメータに対するフェードアウトの時間の設定
     * @param parameterId     パラメータID
     * @param value           フェードアウトにかかる時間[秒]
     */
    setParameterFadeOutTime(parameterId, value) {
      const curves = this._motionData.curves;
      for (let i = 0; i < this._motionData.curveCount; ++i) {
        if (parameterId == curves[i].id) {
          curves[i].fadeOutTime = value;
          return;
        }
      }
    }
    /**
     * パラメータに対するフェードインの時間の取得
     * @param    parameterId     パラメータID
     * @return   フェードインにかかる時間[秒]
     */
    getParameterFadeInTime(parameterId) {
      const curves = this._motionData.curves;
      for (let i = 0; i < this._motionData.curveCount; ++i) {
        if (parameterId == curves[i].id) {
          return curves[i].fadeInTime;
        }
      }
      return -1;
    }
    /**
     * パラメータに対するフェードアウトの時間を取得
     *
     * @param   parameterId     パラメータID
     * @return   フェードアウトにかかる時間[秒]
     */
    getParameterFadeOutTime(parameterId) {
      const curves = this._motionData.curves;
      for (let i = 0; i < this._motionData.curveCount; ++i) {
        if (parameterId == curves[i].id) {
          return curves[i].fadeOutTime;
        }
      }
      return -1;
    }
    /**
     * 自動エフェクトがかかっているパラメータIDリストの設定
     * @param eyeBlinkParameterIds    自動まばたきがかかっているパラメータIDのリスト
     * @param lipSyncParameterIds     リップシンクがかかっているパラメータIDのリスト
     */
    setEffectIds(eyeBlinkParameterIds, lipSyncParameterIds) {
      this._eyeBlinkParameterIds = eyeBlinkParameterIds;
      this._lipSyncParameterIds = lipSyncParameterIds;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this._motionData = void 0;
      this._motionData = null;
    }
    /**
     *
     * @param motionQueueEntry
     * @param userTimeSeconds
     * @param time
     */
    updateForNextLoop(motionQueueEntry, userTimeSeconds, time) {
      switch (this._motionBehavior) {
        case 1 /* MotionBehavior_V2 */:
        default:
          motionQueueEntry.setStartTime(userTimeSeconds - time);
          if (this._isLoopFadeIn) {
            motionQueueEntry.setFadeInStartTime(userTimeSeconds - time);
          }
          if (this._onFinishedMotion != null) {
            this._onFinishedMotion(this);
          }
          break;
        case 0 /* MotionBehavior_V1 */:
          motionQueueEntry.setStartTime(userTimeSeconds);
          if (this._isLoopFadeIn) {
            motionQueueEntry.setFadeInStartTime(userTimeSeconds);
          }
          break;
      }
    }
    /**
     * motion3.jsonをパースする。
     *
     * @param motionJson  motion3.jsonが読み込まれているバッファ
     * @param size        バッファのサイズ
     * @param shouldCheckMotionConsistency motion3.json整合性チェックするかどうか
     */
    parse(motionJson, size, shouldCheckMotionConsistency = false) {
      let json = new CubismMotionJson(motionJson, size);
      if (!json) {
        json.release();
        json = void 0;
        return;
      }
      if (shouldCheckMotionConsistency) {
        const consistency = json.hasConsistency();
        if (!consistency) {
          json.release();
          CubismLogError("Inconsistent motion3.json.");
          return;
        }
      }
      this._motionData = new CubismMotionData();
      this._motionData.duration = json.getMotionDuration();
      this._motionData.loop = json.isMotionLoop();
      this._motionData.curveCount = json.getMotionCurveCount();
      this._motionData.fps = json.getMotionFps();
      this._motionData.eventCount = json.getEventCount();
      const areBeziersRestructed = json.getEvaluationOptionFlag(
        0 /* EvaluationOptionFlag_AreBeziersRistricted */
      );
      if (json.isExistMotionFadeInTime()) {
        this._fadeInSeconds = json.getMotionFadeInTime() < 0 ? 1 : json.getMotionFadeInTime();
      } else {
        this._fadeInSeconds = 1;
      }
      if (json.isExistMotionFadeOutTime()) {
        this._fadeOutSeconds = json.getMotionFadeOutTime() < 0 ? 1 : json.getMotionFadeOutTime();
      } else {
        this._fadeOutSeconds = 1;
      }
      updateSize(
        this._motionData.curves,
        this._motionData.curveCount,
        CubismMotionCurve,
        true
      );
      updateSize(
        this._motionData.segments,
        json.getMotionTotalSegmentCount(),
        CubismMotionSegment,
        true
      );
      updateSize(
        this._motionData.points,
        json.getMotionTotalPointCount(),
        CubismMotionPoint,
        true
      );
      updateSize(
        this._motionData.events,
        this._motionData.eventCount,
        CubismMotionEvent,
        true
      );
      let totalPointCount = 0;
      let totalSegmentCount = 0;
      for (let curveCount = 0; curveCount < this._motionData.curveCount; ++curveCount) {
        if (json.getMotionCurveTarget(curveCount) == TargetNameModel) {
          this._motionData.curves[curveCount].type = 0 /* CubismMotionCurveTarget_Model */;
        } else if (json.getMotionCurveTarget(curveCount) == TargetNameParameter) {
          this._motionData.curves[curveCount].type = 1 /* CubismMotionCurveTarget_Parameter */;
        } else if (json.getMotionCurveTarget(curveCount) == TargetNamePartOpacity) {
          this._motionData.curves[curveCount].type = 2 /* CubismMotionCurveTarget_PartOpacity */;
        } else {
          CubismLogWarning(
            'Warning : Unable to get segment type from Curve! The number of "CurveCount" may be incorrect!'
          );
        }
        this._motionData.curves[curveCount].id = json.getMotionCurveId(curveCount);
        this._motionData.curves[curveCount].baseSegmentIndex = totalSegmentCount;
        this._motionData.curves[curveCount].fadeInTime = json.isExistMotionCurveFadeInTime(curveCount) ? json.getMotionCurveFadeInTime(curveCount) : -1;
        this._motionData.curves[curveCount].fadeOutTime = json.isExistMotionCurveFadeOutTime(curveCount) ? json.getMotionCurveFadeOutTime(curveCount) : -1;
        for (let segmentPosition = 0; segmentPosition < json.getMotionCurveSegmentCount(curveCount); ) {
          if (segmentPosition == 0) {
            this._motionData.segments[totalSegmentCount].basePointIndex = totalPointCount;
            this._motionData.points[totalPointCount].time = json.getMotionCurveSegment(curveCount, segmentPosition);
            this._motionData.points[totalPointCount].value = json.getMotionCurveSegment(curveCount, segmentPosition + 1);
            totalPointCount += 1;
            segmentPosition += 2;
          } else {
            this._motionData.segments[totalSegmentCount].basePointIndex = totalPointCount - 1;
          }
          const segment = json.getMotionCurveSegment(
            curveCount,
            segmentPosition
          );
          const segmentType = segment;
          switch (segmentType) {
            case 0 /* CubismMotionSegmentType_Linear */: {
              this._motionData.segments[totalSegmentCount].segmentType = 0 /* CubismMotionSegmentType_Linear */;
              this._motionData.segments[totalSegmentCount].evaluate = linearEvaluate;
              this._motionData.points[totalPointCount].time = json.getMotionCurveSegment(curveCount, segmentPosition + 1);
              this._motionData.points[totalPointCount].value = json.getMotionCurveSegment(curveCount, segmentPosition + 2);
              totalPointCount += 1;
              segmentPosition += 3;
              break;
            }
            case 1 /* CubismMotionSegmentType_Bezier */: {
              this._motionData.segments[totalSegmentCount].segmentType = 1 /* CubismMotionSegmentType_Bezier */;
              if (areBeziersRestructed || UseOldBeziersCurveMotion) {
                this._motionData.segments[totalSegmentCount].evaluate = bezierEvaluate;
              } else {
                this._motionData.segments[totalSegmentCount].evaluate = bezierEvaluateCardanoInterpretation;
              }
              this._motionData.points[totalPointCount].time = json.getMotionCurveSegment(curveCount, segmentPosition + 1);
              this._motionData.points[totalPointCount].value = json.getMotionCurveSegment(curveCount, segmentPosition + 2);
              this._motionData.points[totalPointCount + 1].time = json.getMotionCurveSegment(curveCount, segmentPosition + 3);
              this._motionData.points[totalPointCount + 1].value = json.getMotionCurveSegment(curveCount, segmentPosition + 4);
              this._motionData.points[totalPointCount + 2].time = json.getMotionCurveSegment(curveCount, segmentPosition + 5);
              this._motionData.points[totalPointCount + 2].value = json.getMotionCurveSegment(curveCount, segmentPosition + 6);
              totalPointCount += 3;
              segmentPosition += 7;
              break;
            }
            case 2 /* CubismMotionSegmentType_Stepped */: {
              this._motionData.segments[totalSegmentCount].segmentType = 2 /* CubismMotionSegmentType_Stepped */;
              this._motionData.segments[totalSegmentCount].evaluate = steppedEvaluate;
              this._motionData.points[totalPointCount].time = json.getMotionCurveSegment(curveCount, segmentPosition + 1);
              this._motionData.points[totalPointCount].value = json.getMotionCurveSegment(curveCount, segmentPosition + 2);
              totalPointCount += 1;
              segmentPosition += 3;
              break;
            }
            case 3 /* CubismMotionSegmentType_InverseStepped */: {
              this._motionData.segments[totalSegmentCount].segmentType = 3 /* CubismMotionSegmentType_InverseStepped */;
              this._motionData.segments[totalSegmentCount].evaluate = inverseSteppedEvaluate;
              this._motionData.points[totalPointCount].time = json.getMotionCurveSegment(curveCount, segmentPosition + 1);
              this._motionData.points[totalPointCount].value = json.getMotionCurveSegment(curveCount, segmentPosition + 2);
              totalPointCount += 1;
              segmentPosition += 3;
              break;
            }
            default: {
              CSM_ASSERT(0);
              break;
            }
          }
          ++this._motionData.curves[curveCount].segmentCount;
          ++totalSegmentCount;
        }
      }
      for (let userdatacount = 0; userdatacount < json.getEventCount(); ++userdatacount) {
        this._motionData.events[userdatacount].fireTime = json.getEventTime(userdatacount);
        this._motionData.events[userdatacount].value = json.getEventValue(userdatacount);
      }
      json.release();
      json = void 0;
      json = null;
    }
    /**
     * モデルのパラメータ更新
     *
     * イベント発火のチェック。
     * 入力する時間は呼ばれるモーションタイミングを０とした秒数で行う。
     *
     * @param beforeCheckTimeSeconds   前回のイベントチェック時間[秒]
     * @param motionTimeSeconds        今回の再生時間[秒]
     */
    getFiredEvent(beforeCheckTimeSeconds, motionTimeSeconds) {
      updateSize(this._firedEventValues, 0);
      for (let u = 0; u < this._motionData.eventCount; ++u) {
        if (this._motionData.events[u].fireTime > beforeCheckTimeSeconds && this._motionData.events[u].fireTime <= motionTimeSeconds) {
          this._firedEventValues.push(this._motionData.events[u].value);
        }
      }
      return this._firedEventValues;
    }
    /**
     * 透明度のカーブが存在するかどうかを確認する
     *
     * @return true  -> キーが存在する
     *          false -> キーが存在しない
     */
    isExistModelOpacity() {
      for (let i = 0; i < this._motionData.curveCount; i++) {
        const curve = this._motionData.curves[i];
        if (curve.type != 0 /* CubismMotionCurveTarget_Model */) {
          continue;
        }
        if (curve.id.getString().localeCompare(IdNameOpacity) == 0) {
          return true;
        }
      }
      return false;
    }
    /**
     * 透明度のカーブのインデックスを返す
     *
     * @return success:透明度のカーブのインデックス
     */
    getModelOpacityIndex() {
      if (this.isExistModelOpacity()) {
        for (let i = 0; i < this._motionData.curveCount; i++) {
          const curve = this._motionData.curves[i];
          if (curve.type != 0 /* CubismMotionCurveTarget_Model */) {
            continue;
          }
          if (curve.id.getString().localeCompare(IdNameOpacity) == 0) {
            return i;
          }
        }
      }
      return -1;
    }
    /**
     * 透明度のIdを返す
     *
     * @param index モーションカーブのインデックス
     * @return success:透明度のカーブのインデックス
     */
    getModelOpacityId(index) {
      if (index != -1) {
        const curve = this._motionData.curves[index];
        if (curve.type == 0 /* CubismMotionCurveTarget_Model */) {
          if (curve.id.getString().localeCompare(IdNameOpacity) == 0) {
            return CubismFramework.getIdManager().getId(curve.id.getString());
          }
        }
      }
      return null;
    }
    /**
     * 現在時間の透明度の値を返す
     *
     * @return success:モーションの当該時間におけるOpacityの値
     */
    getModelOpacityValue() {
      return this._modelOpacity;
    }
    /**
     * デバッグ用フラグを設定する
     *
     * @param debugMode デバッグモードの有効・無効
     */
    setDebugMode(debugMode) {
      this._debugMode = debugMode;
    }
    // デバッグモードかどうか
  };
  var Live2DCubismFramework28;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMotion = CubismMotion;
  })(Live2DCubismFramework28 || (Live2DCubismFramework28 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismmotionmanager.ts
  var CubismMotionManager = class extends CubismMotionQueueManager {
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._currentPriority = 0;
      this._reservePriority = 0;
    }
    /**
     * 再生中のモーションの優先度の取得
     * @return  モーションの優先度
     */
    getCurrentPriority() {
      return this._currentPriority;
    }
    /**
     * 予約中のモーションの優先度を取得する。
     * @return  モーションの優先度
     */
    getReservePriority() {
      return this._reservePriority;
    }
    /**
     * 予約中のモーションの優先度を設定する。
     * @param   val     優先度
     */
    setReservePriority(val) {
      this._reservePriority = val;
    }
    /**
     * 優先度を設定してモーションを開始する。
     *
     * @param motion          モーション
     * @param autoDelete      再生が狩猟したモーションのインスタンスを削除するならtrue
     * @param priority        優先度
     * @return                開始したモーションの識別番号を返す。個別のモーションが終了したか否かを判定するIsFinished()の引数で使用する。開始できない時は「-1」
     */
    startMotionPriority(motion, autoDelete, priority) {
      if (priority == this._reservePriority) {
        this._reservePriority = 0;
      }
      this._currentPriority = priority;
      return super.startMotion(motion, autoDelete);
    }
    /**
     * モーションを更新して、モデルにパラメータ値を反映する。
     *
     * @param model   対象のモデル
     * @param deltaTimeSeconds    デルタ時間[秒]
     * @return  true    更新されている
     * @return  false   更新されていない
     */
    updateMotion(model, deltaTimeSeconds) {
      this._userTimeSeconds += deltaTimeSeconds;
      const updated = super.doUpdateMotion(model, this._userTimeSeconds);
      if (this.isFinished()) {
        this._currentPriority = 0;
      }
      return updated;
    }
    /**
     * モーションを予約する。
     *
     * @param   priority    優先度
     * @return  true    予約できた
     * @return  false   予約できなかった
     */
    reserveMotion(priority) {
      if (priority <= this._reservePriority || priority <= this._currentPriority) {
        return false;
      }
      this._reservePriority = priority;
      return true;
    }
    // 再生予定のモーションの優先度。再生中は0になる。モーションファイルを別スレッドで読み込むときの機能。
  };
  var Live2DCubismFramework29;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMotionManager = CubismMotionManager;
  })(Live2DCubismFramework29 || (Live2DCubismFramework29 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/physics/cubismphysicsinternal.ts
  var CubismPhysicsTargetType = /* @__PURE__ */ ((CubismPhysicsTargetType2) => {
    CubismPhysicsTargetType2[CubismPhysicsTargetType2["CubismPhysicsTargetType_Parameter"] = 0] = "CubismPhysicsTargetType_Parameter";
    return CubismPhysicsTargetType2;
  })(CubismPhysicsTargetType || {});
  var CubismPhysicsSource = /* @__PURE__ */ ((CubismPhysicsSource2) => {
    CubismPhysicsSource2[CubismPhysicsSource2["CubismPhysicsSource_X"] = 0] = "CubismPhysicsSource_X";
    CubismPhysicsSource2[CubismPhysicsSource2["CubismPhysicsSource_Y"] = 1] = "CubismPhysicsSource_Y";
    CubismPhysicsSource2[CubismPhysicsSource2["CubismPhysicsSource_Angle"] = 2] = "CubismPhysicsSource_Angle";
    return CubismPhysicsSource2;
  })(CubismPhysicsSource || {});
  var PhysicsJsonEffectiveForces = class {
    constructor() {
      this.gravity = new CubismVector2(0, 0);
      this.wind = new CubismVector2(0, 0);
    }
    // 風
  };
  var CubismPhysicsParameter = class {
    // 適用先の種類
  };
  var CubismPhysicsNormalization = class {
    // デフォルト値
  };
  var CubismPhysicsParticle = class {
    constructor() {
      this.initialPosition = new CubismVector2(0, 0);
      this.position = new CubismVector2(0, 0);
      this.lastPosition = new CubismVector2(0, 0);
      this.lastGravity = new CubismVector2(0, 0);
      this.force = new CubismVector2(0, 0);
      this.velocity = new CubismVector2(0, 0);
    }
    // 現在の速度
  };
  var CubismPhysicsSubRig = class {
    constructor() {
      this.normalizationPosition = new CubismPhysicsNormalization();
      this.normalizationAngle = new CubismPhysicsNormalization();
    }
    // 正規化された角度
  };
  var CubismPhysicsInput = class {
    constructor() {
      this.source = new CubismPhysicsParameter();
    }
    // 正規化されたパラメータ値の取得関数
  };
  var CubismPhysicsOutput = class {
    constructor() {
      this.destination = new CubismPhysicsParameter();
      this.translationScale = new CubismVector2(0, 0);
    }
    // 物理演算のスケール値の取得関数
  };
  var CubismPhysicsRig = class {
    constructor() {
      this.settings = new Array();
      this.inputs = new Array();
      this.outputs = new Array();
      this.particles = new Array();
      this.gravity = new CubismVector2(0, 0);
      this.wind = new CubismVector2(0, 0);
      this.fps = 0;
    }
    //物理演算動作FPS
  };
  var Live2DCubismFramework30;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismPhysicsInput = CubismPhysicsInput;
    Live2DCubismFramework51.CubismPhysicsNormalization = CubismPhysicsNormalization;
    Live2DCubismFramework51.CubismPhysicsOutput = CubismPhysicsOutput;
    Live2DCubismFramework51.CubismPhysicsParameter = CubismPhysicsParameter;
    Live2DCubismFramework51.CubismPhysicsParticle = CubismPhysicsParticle;
    Live2DCubismFramework51.CubismPhysicsRig = CubismPhysicsRig;
    Live2DCubismFramework51.CubismPhysicsSource = CubismPhysicsSource;
    Live2DCubismFramework51.CubismPhysicsSubRig = CubismPhysicsSubRig;
    Live2DCubismFramework51.CubismPhysicsTargetType = CubismPhysicsTargetType;
    Live2DCubismFramework51.PhysicsJsonEffectiveForces = PhysicsJsonEffectiveForces;
  })(Live2DCubismFramework30 || (Live2DCubismFramework30 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/physics/cubismphysicsjson.ts
  var Position = "Position";
  var X = "X";
  var Y = "Y";
  var Angle = "Angle";
  var Type = "Type";
  var Id3 = "Id";
  var Meta2 = "Meta";
  var EffectiveForces = "EffectiveForces";
  var TotalInputCount = "TotalInputCount";
  var TotalOutputCount = "TotalOutputCount";
  var PhysicsSettingCount = "PhysicsSettingCount";
  var Gravity = "Gravity";
  var Wind = "Wind";
  var VertexCount = "VertexCount";
  var Fps2 = "Fps";
  var PhysicsSettings = "PhysicsSettings";
  var Normalization = "Normalization";
  var Minimum = "Minimum";
  var Maximum = "Maximum";
  var Default = "Default";
  var Reflect2 = "Reflect";
  var Weight = "Weight";
  var Input = "Input";
  var Source = "Source";
  var Output = "Output";
  var Scale = "Scale";
  var VertexIndex = "VertexIndex";
  var Destination = "Destination";
  var Vertices = "Vertices";
  var Mobility = "Mobility";
  var Delay = "Delay";
  var Radius = "Radius";
  var Acceleration = "Acceleration";
  var CubismPhysicsJson = class {
    /**
     * コンストラクタ
     * @param buffer physics3.jsonが読み込まれているバッファ
     * @param size バッファのサイズ
     */
    constructor(buffer, size) {
      this._json = CubismJson.create(buffer, size);
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      CubismJson.delete(this._json);
    }
    /**
     * 重力の取得
     * @return 重力
     */
    getGravity() {
      const ret = new CubismVector2(0, 0);
      ret.x = this._json.getRoot().getValueByString(Meta2).getValueByString(EffectiveForces).getValueByString(Gravity).getValueByString(X).toFloat();
      ret.y = this._json.getRoot().getValueByString(Meta2).getValueByString(EffectiveForces).getValueByString(Gravity).getValueByString(Y).toFloat();
      return ret;
    }
    /**
     * 風の取得
     * @return 風
     */
    getWind() {
      const ret = new CubismVector2(0, 0);
      ret.x = this._json.getRoot().getValueByString(Meta2).getValueByString(EffectiveForces).getValueByString(Wind).getValueByString(X).toFloat();
      ret.y = this._json.getRoot().getValueByString(Meta2).getValueByString(EffectiveForces).getValueByString(Wind).getValueByString(Y).toFloat();
      return ret;
    }
    /**
     * 物理演算設定FPSの取得
     * @return 物理演算設定FPS
     */
    getFps() {
      return this._json.getRoot().getValueByString(Meta2).getValueByString(Fps2).toFloat(0);
    }
    /**
     * 物理店の管理の個数の取得
     * @return 物理店の管理の個数
     */
    getSubRigCount() {
      return this._json.getRoot().getValueByString(Meta2).getValueByString(PhysicsSettingCount).toInt();
    }
    /**
     * 入力の総合計の取得
     * @return 入力の総合計
     */
    getTotalInputCount() {
      return this._json.getRoot().getValueByString(Meta2).getValueByString(TotalInputCount).toInt();
    }
    /**
     * 出力の総合計の取得
     * @return 出力の総合計
     */
    getTotalOutputCount() {
      return this._json.getRoot().getValueByString(Meta2).getValueByString(TotalOutputCount).toInt();
    }
    /**
     * 物理点の個数の取得
     * @return 物理点の個数
     */
    getVertexCount() {
      return this._json.getRoot().getValueByString(Meta2).getValueByString(VertexCount).toInt();
    }
    /**
     * 正規化された位置の最小値の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 正規化された位置の最小値
     */
    getNormalizationPositionMinimumValue(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Normalization).getValueByString(Position).getValueByString(Minimum).toFloat();
    }
    /**
     * 正規化された位置の最大値の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 正規化された位置の最大値
     */
    getNormalizationPositionMaximumValue(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Normalization).getValueByString(Position).getValueByString(Maximum).toFloat();
    }
    /**
     * 正規化された位置のデフォルト値の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 正規化された位置のデフォルト値
     */
    getNormalizationPositionDefaultValue(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Normalization).getValueByString(Position).getValueByString(Default).toFloat();
    }
    /**
     * 正規化された角度の最小値の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 正規化された角度の最小値
     */
    getNormalizationAngleMinimumValue(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Normalization).getValueByString(Angle).getValueByString(Minimum).toFloat();
    }
    /**
     * 正規化された角度の最大値の取得
     * @param physicsSettingIndex
     * @return 正規化された角度の最大値
     */
    getNormalizationAngleMaximumValue(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Normalization).getValueByString(Angle).getValueByString(Maximum).toFloat();
    }
    /**
     * 正規化された角度のデフォルト値の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 正規化された角度のデフォルト値
     */
    getNormalizationAngleDefaultValue(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Normalization).getValueByString(Angle).getValueByString(Default).toFloat();
    }
    /**
     * 入力の個数の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 入力の個数
     */
    getInputCount(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Input).getVector().length;
    }
    /**
     * 入力の重みの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param inputIndex 入力のインデックス
     * @return 入力の重み
     */
    getInputWeight(physicsSettingIndex, inputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Input).getValueByIndex(inputIndex).getValueByString(Weight).toFloat();
    }
    /**
     * 入力の反転の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param inputIndex 入力のインデックス
     * @return 入力の反転
     */
    getInputReflect(physicsSettingIndex, inputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Input).getValueByIndex(inputIndex).getValueByString(Reflect2).toBoolean();
    }
    /**
     * 入力の種類の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param inputIndex 入力のインデックス
     * @return 入力の種類
     */
    getInputType(physicsSettingIndex, inputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Input).getValueByIndex(inputIndex).getValueByString(Type).getRawString();
    }
    /**
     * 入力元のIDの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param inputIndex 入力のインデックス
     * @return 入力元のID
     */
    getInputSourceId(physicsSettingIndex, inputIndex) {
      return CubismFramework.getIdManager().getId(
        this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Input).getValueByIndex(inputIndex).getValueByString(Source).getValueByString(Id3).getRawString()
      );
    }
    /**
     * 出力の個数の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @return 出力の個数
     */
    getOutputCount(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getVector().length;
    }
    /**
     * 出力の物理点のインデックスの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param outputIndex 出力のインデックス
     * @return 出力の物理点のインデックス
     */
    getOutputVertexIndex(physicsSettingIndex, outputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getValueByIndex(outputIndex).getValueByString(VertexIndex).toInt();
    }
    /**
     * 出力の角度のスケールを取得する
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param outputIndex 出力のインデックス
     * @return 出力の角度のスケール
     */
    getOutputAngleScale(physicsSettingIndex, outputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getValueByIndex(outputIndex).getValueByString(Scale).toFloat();
    }
    /**
     * 出力の重みの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param outputIndex 出力のインデックス
     * @return 出力の重み
     */
    getOutputWeight(physicsSettingIndex, outputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getValueByIndex(outputIndex).getValueByString(Weight).toFloat();
    }
    /**
     * 出力先のIDの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param outputIndex 出力のインデックス
     * @return 出力先のID
     */
    getOutputDestinationId(physicsSettingIndex, outputIndex) {
      return CubismFramework.getIdManager().getId(
        this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getValueByIndex(outputIndex).getValueByString(Destination).getValueByString(Id3).getRawString()
      );
    }
    /**
     * 出力の種類の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param outputIndex 出力のインデックス
     * @return 出力の種類
     */
    getOutputType(physicsSettingIndex, outputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getValueByIndex(outputIndex).getValueByString(Type).getRawString();
    }
    /**
     * 出力の反転の取得
     * @param physicsSettingIndex 物理演算のインデックス
     * @param outputIndex 出力のインデックス
     * @return 出力の反転
     */
    getOutputReflect(physicsSettingIndex, outputIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Output).getValueByIndex(outputIndex).getValueByString(Reflect2).toBoolean();
    }
    /**
     * 物理点の個数の取得
     * @param physicsSettingIndex 物理演算男設定のインデックス
     * @return 物理点の個数
     */
    getParticleCount(physicsSettingIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getVector().length;
    }
    /**
     * 物理点の動きやすさの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param vertexIndex 物理点のインデックス
     * @return 物理点の動きやすさ
     */
    getParticleMobility(physicsSettingIndex, vertexIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getValueByIndex(vertexIndex).getValueByString(Mobility).toFloat();
    }
    /**
     * 物理点の遅れの取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param vertexIndex 物理点のインデックス
     * @return 物理点の遅れ
     */
    getParticleDelay(physicsSettingIndex, vertexIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getValueByIndex(vertexIndex).getValueByString(Delay).toFloat();
    }
    /**
     * 物理点の加速度の取得
     * @param physicsSettingIndex 物理演算の設定
     * @param vertexIndex 物理点のインデックス
     * @return 物理点の加速度
     */
    getParticleAcceleration(physicsSettingIndex, vertexIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getValueByIndex(vertexIndex).getValueByString(Acceleration).toFloat();
    }
    /**
     * 物理点の距離の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param vertexIndex 物理点のインデックス
     * @return 物理点の距離
     */
    getParticleRadius(physicsSettingIndex, vertexIndex) {
      return this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getValueByIndex(vertexIndex).getValueByString(Radius).toFloat();
    }
    /**
     * 物理点の位置の取得
     * @param physicsSettingIndex 物理演算の設定のインデックス
     * @param vertexInde 物理点のインデックス
     * @return 物理点の位置
     */
    getParticlePosition(physicsSettingIndex, vertexIndex) {
      const ret = new CubismVector2(0, 0);
      ret.x = this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getValueByIndex(vertexIndex).getValueByString(Position).getValueByString(X).toFloat();
      ret.y = this._json.getRoot().getValueByString(PhysicsSettings).getValueByIndex(physicsSettingIndex).getValueByString(Vertices).getValueByIndex(vertexIndex).getValueByString(Position).getValueByString(Y).toFloat();
      return ret;
    }
    // physics3.jsonデータ
  };
  var Live2DCubismFramework31;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismPhysicsJson = CubismPhysicsJson;
  })(Live2DCubismFramework31 || (Live2DCubismFramework31 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/physics/cubismphysics.ts
  var PhysicsTypeTagX = "X";
  var PhysicsTypeTagY = "Y";
  var PhysicsTypeTagAngle = "Angle";
  var AirResistance = 5;
  var MaximumWeight = 100;
  var MovementThreshold = 1e-3;
  var MaxDeltaTime = 5;
  var CubismPhysics = class _CubismPhysics {
    /**
     * インスタンスの作成
     * @param buffer    physics3.jsonが読み込まれているバッファ
     * @param size      バッファのサイズ
     * @return 作成されたインスタンス
     */
    static create(buffer, size) {
      const ret = new _CubismPhysics();
      ret.parse(buffer, size);
      ret._physicsRig.gravity.y = 0;
      return ret;
    }
    /**
     * インスタンスを破棄する
     * @param physics 破棄するインスタンス
     */
    static delete(physics) {
      if (physics != null) {
        physics.release();
        physics = null;
      }
    }
    /**
     * physics3.jsonをパースする。
     * @param physicsJson physics3.jsonが読み込まれているバッファ
     * @param size バッファのサイズ
     */
    parse(physicsJson, size) {
      this._physicsRig = new CubismPhysicsRig();
      let json = new CubismPhysicsJson(physicsJson, size);
      this._physicsRig.gravity = json.getGravity();
      this._physicsRig.wind = json.getWind();
      this._physicsRig.subRigCount = json.getSubRigCount();
      this._physicsRig.fps = json.getFps();
      updateSize(
        this._physicsRig.settings,
        this._physicsRig.subRigCount,
        CubismPhysicsSubRig,
        true
      );
      updateSize(
        this._physicsRig.inputs,
        json.getTotalInputCount(),
        CubismPhysicsInput,
        true
      );
      updateSize(
        this._physicsRig.outputs,
        json.getTotalOutputCount(),
        CubismPhysicsOutput,
        true
      );
      updateSize(
        this._physicsRig.particles,
        json.getVertexCount(),
        CubismPhysicsParticle,
        true
      );
      this._currentRigOutputs.length = 0;
      this._previousRigOutputs.length = 0;
      let inputIndex = 0, outputIndex = 0, particleIndex = 0;
      let dstIndexCurrentRigOutputs = this._currentRigOutputs.length;
      let dstIndexPreviousRigOutputs = this._previousRigOutputs.length;
      this._currentRigOutputs.length += this._physicsRig.settings.length;
      this._previousRigOutputs.length += this._physicsRig.settings.length;
      for (let i = 0; i < this._physicsRig.settings.length; ++i) {
        this._physicsRig.settings[i].normalizationPosition.minimum = json.getNormalizationPositionMinimumValue(i);
        this._physicsRig.settings[i].normalizationPosition.maximum = json.getNormalizationPositionMaximumValue(i);
        this._physicsRig.settings[i].normalizationPosition.defalut = json.getNormalizationPositionDefaultValue(i);
        this._physicsRig.settings[i].normalizationAngle.minimum = json.getNormalizationAngleMinimumValue(i);
        this._physicsRig.settings[i].normalizationAngle.maximum = json.getNormalizationAngleMaximumValue(i);
        this._physicsRig.settings[i].normalizationAngle.defalut = json.getNormalizationAngleDefaultValue(i);
        this._physicsRig.settings[i].inputCount = json.getInputCount(i);
        this._physicsRig.settings[i].baseInputIndex = inputIndex;
        for (let j = 0; j < this._physicsRig.settings[i].inputCount; ++j) {
          this._physicsRig.inputs[inputIndex + j].sourceParameterIndex = -1;
          this._physicsRig.inputs[inputIndex + j].weight = json.getInputWeight(
            i,
            j
          );
          this._physicsRig.inputs[inputIndex + j].reflect = json.getInputReflect(
            i,
            j
          );
          if (json.getInputType(i, j) == PhysicsTypeTagX) {
            this._physicsRig.inputs[inputIndex + j].type = 0 /* CubismPhysicsSource_X */;
            this._physicsRig.inputs[inputIndex + j].getNormalizedParameterValue = getInputTranslationXFromNormalizedParameterValue;
          } else if (json.getInputType(i, j) == PhysicsTypeTagY) {
            this._physicsRig.inputs[inputIndex + j].type = 1 /* CubismPhysicsSource_Y */;
            this._physicsRig.inputs[inputIndex + j].getNormalizedParameterValue = getInputTranslationYFromNormalizedParamterValue;
          } else if (json.getInputType(i, j) == PhysicsTypeTagAngle) {
            this._physicsRig.inputs[inputIndex + j].type = 2 /* CubismPhysicsSource_Angle */;
            this._physicsRig.inputs[inputIndex + j].getNormalizedParameterValue = getInputAngleFromNormalizedParameterValue;
          }
          this._physicsRig.inputs[inputIndex + j].source.targetType = 0 /* CubismPhysicsTargetType_Parameter */;
          this._physicsRig.inputs[inputIndex + j].source.id = json.getInputSourceId(i, j);
        }
        inputIndex += this._physicsRig.settings[i].inputCount;
        this._physicsRig.settings[i].outputCount = json.getOutputCount(i);
        this._physicsRig.settings[i].baseOutputIndex = outputIndex;
        const currentRigOutput = new PhysicsOutput();
        updateSize(
          currentRigOutput.outputs,
          this._physicsRig.settings[i].outputCount,
          null,
          true
        );
        const previousRigOutput = new PhysicsOutput();
        updateSize(
          previousRigOutput.outputs,
          this._physicsRig.settings[i].outputCount,
          null,
          true
        );
        for (let j = 0; j < this._physicsRig.settings[i].outputCount; ++j) {
          currentRigOutput.outputs[j] = 0;
          previousRigOutput.outputs[j] = 0;
          this._physicsRig.outputs[outputIndex + j].destinationParameterIndex = -1;
          this._physicsRig.outputs[outputIndex + j].vertexIndex = json.getOutputVertexIndex(i, j);
          this._physicsRig.outputs[outputIndex + j].angleScale = json.getOutputAngleScale(i, j);
          this._physicsRig.outputs[outputIndex + j].weight = json.getOutputWeight(
            i,
            j
          );
          this._physicsRig.outputs[outputIndex + j].destination.targetType = 0 /* CubismPhysicsTargetType_Parameter */;
          this._physicsRig.outputs[outputIndex + j].destination.id = json.getOutputDestinationId(i, j);
          if (json.getOutputType(i, j) == PhysicsTypeTagX) {
            this._physicsRig.outputs[outputIndex + j].type = 0 /* CubismPhysicsSource_X */;
            this._physicsRig.outputs[outputIndex + j].getValue = getOutputTranslationX;
            this._physicsRig.outputs[outputIndex + j].getScale = getOutputScaleTranslationX;
          } else if (json.getOutputType(i, j) == PhysicsTypeTagY) {
            this._physicsRig.outputs[outputIndex + j].type = 1 /* CubismPhysicsSource_Y */;
            this._physicsRig.outputs[outputIndex + j].getValue = getOutputTranslationY;
            this._physicsRig.outputs[outputIndex + j].getScale = getOutputScaleTranslationY;
          } else if (json.getOutputType(i, j) == PhysicsTypeTagAngle) {
            this._physicsRig.outputs[outputIndex + j].type = 2 /* CubismPhysicsSource_Angle */;
            this._physicsRig.outputs[outputIndex + j].getValue = getOutputAngle;
            this._physicsRig.outputs[outputIndex + j].getScale = getOutputScaleAngle;
          }
          this._physicsRig.outputs[outputIndex + j].reflect = json.getOutputReflect(i, j);
        }
        this._currentRigOutputs[dstIndexCurrentRigOutputs++] = currentRigOutput;
        this._previousRigOutputs[dstIndexPreviousRigOutputs++] = previousRigOutput;
        outputIndex += this._physicsRig.settings[i].outputCount;
        this._physicsRig.settings[i].particleCount = json.getParticleCount(i);
        this._physicsRig.settings[i].baseParticleIndex = particleIndex;
        for (let j = 0; j < this._physicsRig.settings[i].particleCount; ++j) {
          this._physicsRig.particles[particleIndex + j].mobility = json.getParticleMobility(i, j);
          this._physicsRig.particles[particleIndex + j].delay = json.getParticleDelay(i, j);
          this._physicsRig.particles[particleIndex + j].acceleration = json.getParticleAcceleration(i, j);
          this._physicsRig.particles[particleIndex + j].radius = json.getParticleRadius(i, j);
          this._physicsRig.particles[particleIndex + j].position = json.getParticlePosition(i, j);
        }
        particleIndex += this._physicsRig.settings[i].particleCount;
      }
      this.initialize();
      json.release();
      json = void 0;
      json = null;
    }
    /**
     * 現在のパラメータ値で物理演算が安定化する状態を演算する。
     * @param model 物理演算の結果を適用するモデル
     */
    stabilization(model) {
      var _a, _b, _c, _d;
      let totalAngle;
      let weight;
      let radAngle;
      let outputValue;
      const totalTranslation = new CubismVector2();
      let currentSetting;
      let currentInputs;
      let currentOutputs;
      let currentParticles;
      const parameterValues = model.getModel().parameters.values;
      const parameterMaximumValues = model.getModel().parameters.maximumValues;
      const parameterMinimumValues = model.getModel().parameters.minimumValues;
      const parameterDefaultValues = model.getModel().parameters.defaultValues;
      if (((_b = (_a = this._parameterCaches) == null ? void 0 : _a.length) != null ? _b : 0) < model.getParameterCount()) {
        this._parameterCaches = new Float32Array(model.getParameterCount());
      }
      if (((_d = (_c = this._parameterInputCaches) == null ? void 0 : _c.length) != null ? _d : 0) < model.getParameterCount()) {
        this._parameterInputCaches = new Float32Array(model.getParameterCount());
      }
      for (let j = 0; j < model.getParameterCount(); ++j) {
        this._parameterCaches[j] = parameterValues[j];
        this._parameterInputCaches[j] = parameterValues[j];
      }
      for (let settingIndex = 0; settingIndex < this._physicsRig.subRigCount; ++settingIndex) {
        totalAngle = { angle: 0 };
        totalTranslation.x = 0;
        totalTranslation.y = 0;
        currentSetting = this._physicsRig.settings[settingIndex];
        currentInputs = this._physicsRig.inputs.slice(
          currentSetting.baseInputIndex
        );
        currentOutputs = this._physicsRig.outputs.slice(
          currentSetting.baseOutputIndex
        );
        currentParticles = this._physicsRig.particles.slice(
          currentSetting.baseParticleIndex
        );
        for (let i = 0; i < currentSetting.inputCount; ++i) {
          weight = currentInputs[i].weight / MaximumWeight;
          if (currentInputs[i].sourceParameterIndex == -1) {
            currentInputs[i].sourceParameterIndex = model.getParameterIndex(
              currentInputs[i].source.id
            );
          }
          currentInputs[i].getNormalizedParameterValue(
            totalTranslation,
            totalAngle,
            parameterValues[currentInputs[i].sourceParameterIndex],
            parameterMinimumValues[currentInputs[i].sourceParameterIndex],
            parameterMaximumValues[currentInputs[i].sourceParameterIndex],
            parameterDefaultValues[currentInputs[i].sourceParameterIndex],
            currentSetting.normalizationPosition,
            currentSetting.normalizationAngle,
            currentInputs[i].reflect,
            weight
          );
          this._parameterCaches[currentInputs[i].sourceParameterIndex] = parameterValues[currentInputs[i].sourceParameterIndex];
        }
        radAngle = CubismMath.degreesToRadian(-totalAngle.angle);
        totalTranslation.x = totalTranslation.x * CubismMath.cos(radAngle) - totalTranslation.y * CubismMath.sin(radAngle);
        totalTranslation.y = totalTranslation.x * CubismMath.sin(radAngle) + totalTranslation.y * CubismMath.cos(radAngle);
        updateParticlesForStabilization(
          currentParticles,
          currentSetting.particleCount,
          totalTranslation,
          totalAngle.angle,
          this._options.wind,
          MovementThreshold * currentSetting.normalizationPosition.maximum
        );
        for (let i = 0; i < currentSetting.outputCount; ++i) {
          const particleIndex = currentOutputs[i].vertexIndex;
          if (currentOutputs[i].destinationParameterIndex == -1) {
            currentOutputs[i].destinationParameterIndex = model.getParameterIndex(
              currentOutputs[i].destination.id
            );
          }
          if (particleIndex < 1 || particleIndex >= currentSetting.particleCount) {
            continue;
          }
          let translation = new CubismVector2();
          translation = currentParticles[particleIndex].position.substract(
            currentParticles[particleIndex - 1].position
          );
          outputValue = currentOutputs[i].getValue(
            translation,
            currentParticles,
            particleIndex,
            currentOutputs[i].reflect,
            this._options.gravity
          );
          this._currentRigOutputs[settingIndex].outputs[i] = outputValue;
          this._previousRigOutputs[settingIndex].outputs[i] = outputValue;
          const destinationParameterIndex = currentOutputs[i].destinationParameterIndex;
          const outParameterCaches = !Float32Array.prototype.slice && "subarray" in Float32Array.prototype ? JSON.parse(
            JSON.stringify(
              parameterValues.subarray(destinationParameterIndex)
            )
          ) : parameterValues.slice(destinationParameterIndex);
          updateOutputParameterValue(
            outParameterCaches,
            parameterMinimumValues[destinationParameterIndex],
            parameterMaximumValues[destinationParameterIndex],
            outputValue,
            currentOutputs[i]
          );
          for (let offset = destinationParameterIndex, outParamIndex = 0; offset < this._parameterCaches.length; offset++, outParamIndex++) {
            parameterValues[offset] = this._parameterCaches[offset] = outParameterCaches[outParamIndex];
          }
        }
      }
    }
    /**
     * 物理演算の評価
     *
     * Pendulum interpolation weights
     *
     * 振り子の計算結果は保存され、パラメータへの出力は保存された前回の結果で補間されます。
     * The result of the pendulum calculation is saved and
     * the output to the parameters is interpolated with the saved previous result of the pendulum calculation.
     *
     * 図で示すと[1]と[2]で補間されます。
     * The figure shows the interpolation between [1] and [2].
     *
     * 補間の重みは最新の振り子計算タイミングと次回のタイミングの間で見た現在時間で決定する。
     * The weight of the interpolation are determined by the current time seen between
     * the latest pendulum calculation timing and the next timing.
     *
     * 図で示すと[2]と[4]の間でみた(3)の位置の重みになる。
     * Figure shows the weight of position (3) as seen between [2] and [4].
     *
     * 解釈として振り子計算のタイミングと重み計算のタイミングがズレる。
     * As an interpretation, the pendulum calculation and weights are misaligned.
     *
     * physics3.jsonにFPS情報が存在しない場合は常に前の振り子状態で設定される。
     * If there is no FPS information in physics3.json, it is always set in the previous pendulum state.
     *
     * この仕様は補間範囲を逸脱したことが原因の震えたような見た目を回避を目的にしている。
     * The purpose of this specification is to avoid the quivering appearance caused by deviations from the interpolation range.
     *
     * ------------ time -------------->
     *
     *                 |+++++|------| <- weight
     * ==[1]====#=====[2]---(3)----(4)
     *          ^ output contents
     *
     * 1:_previousRigOutputs
     * 2:_currentRigOutputs
     * 3:_currentRemainTime (now rendering)
     * 4:next particles timing
     * @param model 物理演算の結果を適用するモデル
     * @param deltaTimeSeconds デルタ時間[秒]
     */
    evaluate(model, deltaTimeSeconds) {
      var _a, _b, _c, _d;
      let totalAngle;
      let weight;
      let radAngle;
      let outputValue;
      const totalTranslation = new CubismVector2();
      let currentSetting;
      let currentInputs;
      let currentOutputs;
      let currentParticles;
      if (0 >= deltaTimeSeconds) {
        return;
      }
      const parameterValues = model.getModel().parameters.values;
      const parameterMaximumValues = model.getModel().parameters.maximumValues;
      const parameterMinimumValues = model.getModel().parameters.minimumValues;
      const parameterDefaultValues = model.getModel().parameters.defaultValues;
      let physicsDeltaTime;
      this._currentRemainTime += deltaTimeSeconds;
      if (this._currentRemainTime > MaxDeltaTime) {
        this._currentRemainTime = 0;
      }
      if (((_b = (_a = this._parameterCaches) == null ? void 0 : _a.length) != null ? _b : 0) < model.getParameterCount()) {
        this._parameterCaches = new Float32Array(model.getParameterCount());
      }
      if (((_d = (_c = this._parameterInputCaches) == null ? void 0 : _c.length) != null ? _d : 0) < model.getParameterCount()) {
        this._parameterInputCaches = new Float32Array(model.getParameterCount());
        for (let j = 0; j < model.getParameterCount(); ++j) {
          this._parameterInputCaches[j] = parameterValues[j];
        }
      }
      if (this._physicsRig.fps > 0) {
        physicsDeltaTime = 1 / this._physicsRig.fps;
      } else {
        physicsDeltaTime = deltaTimeSeconds;
      }
      while (this._currentRemainTime >= physicsDeltaTime) {
        for (let settingIndex = 0; settingIndex < this._physicsRig.subRigCount; ++settingIndex) {
          currentSetting = this._physicsRig.settings[settingIndex];
          currentOutputs = this._physicsRig.outputs.slice(
            currentSetting.baseOutputIndex
          );
          for (let i = 0; i < currentSetting.outputCount; ++i) {
            this._previousRigOutputs[settingIndex].outputs[i] = this._currentRigOutputs[settingIndex].outputs[i];
          }
        }
        const inputWeight = physicsDeltaTime / this._currentRemainTime;
        for (let j = 0; j < model.getParameterCount(); ++j) {
          this._parameterCaches[j] = this._parameterInputCaches[j] * (1 - inputWeight) + parameterValues[j] * inputWeight;
          this._parameterInputCaches[j] = this._parameterCaches[j];
        }
        for (let settingIndex = 0; settingIndex < this._physicsRig.subRigCount; ++settingIndex) {
          totalAngle = { angle: 0 };
          totalTranslation.x = 0;
          totalTranslation.y = 0;
          currentSetting = this._physicsRig.settings[settingIndex];
          currentInputs = this._physicsRig.inputs.slice(
            currentSetting.baseInputIndex
          );
          currentOutputs = this._physicsRig.outputs.slice(
            currentSetting.baseOutputIndex
          );
          currentParticles = this._physicsRig.particles.slice(
            currentSetting.baseParticleIndex
          );
          for (let i = 0; i < currentSetting.inputCount; ++i) {
            weight = currentInputs[i].weight / MaximumWeight;
            if (currentInputs[i].sourceParameterIndex == -1) {
              currentInputs[i].sourceParameterIndex = model.getParameterIndex(
                currentInputs[i].source.id
              );
            }
            currentInputs[i].getNormalizedParameterValue(
              totalTranslation,
              totalAngle,
              this._parameterCaches[currentInputs[i].sourceParameterIndex],
              parameterMinimumValues[currentInputs[i].sourceParameterIndex],
              parameterMaximumValues[currentInputs[i].sourceParameterIndex],
              parameterDefaultValues[currentInputs[i].sourceParameterIndex],
              currentSetting.normalizationPosition,
              currentSetting.normalizationAngle,
              currentInputs[i].reflect,
              weight
            );
          }
          radAngle = CubismMath.degreesToRadian(-totalAngle.angle);
          totalTranslation.x = totalTranslation.x * CubismMath.cos(radAngle) - totalTranslation.y * CubismMath.sin(radAngle);
          totalTranslation.y = totalTranslation.x * CubismMath.sin(radAngle) + totalTranslation.y * CubismMath.cos(radAngle);
          updateParticles(
            currentParticles,
            currentSetting.particleCount,
            totalTranslation,
            totalAngle.angle,
            this._options.wind,
            MovementThreshold * currentSetting.normalizationPosition.maximum,
            physicsDeltaTime,
            AirResistance
          );
          for (let i = 0; i < currentSetting.outputCount; ++i) {
            const particleIndex = currentOutputs[i].vertexIndex;
            if (currentOutputs[i].destinationParameterIndex == -1) {
              currentOutputs[i].destinationParameterIndex = model.getParameterIndex(currentOutputs[i].destination.id);
            }
            if (particleIndex < 1 || particleIndex >= currentSetting.particleCount) {
              continue;
            }
            const translation = new CubismVector2();
            translation.x = currentParticles[particleIndex].position.x - currentParticles[particleIndex - 1].position.x;
            translation.y = currentParticles[particleIndex].position.y - currentParticles[particleIndex - 1].position.y;
            outputValue = currentOutputs[i].getValue(
              translation,
              currentParticles,
              particleIndex,
              currentOutputs[i].reflect,
              this._options.gravity
            );
            this._currentRigOutputs[settingIndex].outputs[i] = outputValue;
            const destinationParameterIndex = currentOutputs[i].destinationParameterIndex;
            const outParameterCaches = !Float32Array.prototype.slice && "subarray" in Float32Array.prototype ? JSON.parse(
              JSON.stringify(
                this._parameterCaches.subarray(destinationParameterIndex)
              )
            ) : this._parameterCaches.slice(destinationParameterIndex);
            updateOutputParameterValue(
              outParameterCaches,
              parameterMinimumValues[destinationParameterIndex],
              parameterMaximumValues[destinationParameterIndex],
              outputValue,
              currentOutputs[i]
            );
            for (let offset = destinationParameterIndex, outParamIndex = 0; offset < this._parameterCaches.length; offset++, outParamIndex++) {
              this._parameterCaches[offset] = outParameterCaches[outParamIndex];
            }
          }
        }
        this._currentRemainTime -= physicsDeltaTime;
      }
      const alpha = this._currentRemainTime / physicsDeltaTime;
      this.interpolate(model, alpha);
    }
    /**
     * 物理演算結果の適用
     * 振り子演算の最新の結果と一つ前の結果から指定した重みで適用する。
     * @param model 物理演算の結果を適用するモデル
     * @param weight 最新結果の重み
     */
    interpolate(model, weight) {
      let currentOutputs;
      let currentSetting;
      const parameterValues = model.getModel().parameters.values;
      const parameterMaximumValues = model.getModel().parameters.maximumValues;
      const parameterMinimumValues = model.getModel().parameters.minimumValues;
      for (let settingIndex = 0; settingIndex < this._physicsRig.subRigCount; ++settingIndex) {
        currentSetting = this._physicsRig.settings[settingIndex];
        currentOutputs = this._physicsRig.outputs.slice(
          currentSetting.baseOutputIndex
        );
        for (let i = 0; i < currentSetting.outputCount; ++i) {
          if (currentOutputs[i].destinationParameterIndex == -1) {
            continue;
          }
          const destinationParameterIndex = currentOutputs[i].destinationParameterIndex;
          const outParameterValues = !Float32Array.prototype.slice && "subarray" in Float32Array.prototype ? JSON.parse(
            JSON.stringify(
              parameterValues.subarray(destinationParameterIndex)
            )
          ) : parameterValues.slice(destinationParameterIndex);
          updateOutputParameterValue(
            outParameterValues,
            parameterMinimumValues[destinationParameterIndex],
            parameterMaximumValues[destinationParameterIndex],
            this._previousRigOutputs[settingIndex].outputs[i] * (1 - weight) + this._currentRigOutputs[settingIndex].outputs[i] * weight,
            currentOutputs[i]
          );
          for (let offset = destinationParameterIndex, outParamIndex = 0; offset < parameterValues.length; offset++, outParamIndex++) {
            parameterValues[offset] = outParameterValues[outParamIndex];
          }
        }
      }
    }
    /**
     * オプションの設定
     * @param options オプション
     */
    setOptions(options) {
      this._options = options;
    }
    /**
     * オプションの取得
     * @return オプション
     */
    getOption() {
      return this._options;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._physicsRig = null;
      this._options = new Options();
      this._options.gravity.y = -1;
      this._options.gravity.x = 0;
      this._options.wind.x = 0;
      this._options.wind.y = 0;
      this._currentRigOutputs = new Array();
      this._previousRigOutputs = new Array();
      this._currentRemainTime = 0;
      this._parameterCaches = null;
      this._parameterInputCaches = null;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this._physicsRig = void 0;
      this._physicsRig = null;
    }
    /**
     * 初期化する
     */
    initialize() {
      let strand;
      let currentSetting;
      let radius;
      for (let settingIndex = 0; settingIndex < this._physicsRig.subRigCount; ++settingIndex) {
        currentSetting = this._physicsRig.settings[settingIndex];
        strand = this._physicsRig.particles.slice(
          currentSetting.baseParticleIndex
        );
        strand[0].initialPosition = new CubismVector2(0, 0);
        strand[0].lastPosition = new CubismVector2(
          strand[0].initialPosition.x,
          strand[0].initialPosition.y
        );
        strand[0].lastGravity = new CubismVector2(0, -1);
        strand[0].lastGravity.y *= -1;
        strand[0].velocity = new CubismVector2(0, 0);
        strand[0].force = new CubismVector2(0, 0);
        for (let i = 1; i < currentSetting.particleCount; ++i) {
          radius = new CubismVector2(0, 0);
          radius.y = strand[i].radius;
          strand[i].initialPosition = new CubismVector2(
            strand[i - 1].initialPosition.x + radius.x,
            strand[i - 1].initialPosition.y + radius.y
          );
          strand[i].position = new CubismVector2(
            strand[i].initialPosition.x,
            strand[i].initialPosition.y
          );
          strand[i].lastPosition = new CubismVector2(
            strand[i].initialPosition.x,
            strand[i].initialPosition.y
          );
          strand[i].lastGravity = new CubismVector2(0, -1);
          strand[i].lastGravity.y *= -1;
          strand[i].velocity = new CubismVector2(0, 0);
          strand[i].force = new CubismVector2(0, 0);
        }
      }
    }
    ///< UpdateParticlesが動くときの入力をキャッシュ
  };
  var Options = class {
    constructor() {
      this.gravity = new CubismVector2(0, 0);
      this.wind = new CubismVector2(0, 0);
    }
    // 風の方向
  };
  var PhysicsOutput = class {
    constructor() {
      this.outputs = new Array(0);
    }
    // 物理演算出力結果
  };
  function sign(value) {
    let ret = 0;
    if (value > 0) {
      ret = 1;
    } else if (value < 0) {
      ret = -1;
    }
    return ret;
  }
  function getInputTranslationXFromNormalizedParameterValue(targetTranslation, targetAngle, value, parameterMinimumValue, parameterMaximumValue, parameterDefaultValue, normalizationPosition, normalizationAngle, isInverted, weight) {
    targetTranslation.x += normalizeParameterValue(
      value,
      parameterMinimumValue,
      parameterMaximumValue,
      parameterDefaultValue,
      normalizationPosition.minimum,
      normalizationPosition.maximum,
      normalizationPosition.defalut,
      isInverted
    ) * weight;
  }
  function getInputTranslationYFromNormalizedParamterValue(targetTranslation, targetAngle, value, parameterMinimumValue, parameterMaximumValue, parameterDefaultValue, normalizationPosition, normalizationAngle, isInverted, weight) {
    targetTranslation.y += normalizeParameterValue(
      value,
      parameterMinimumValue,
      parameterMaximumValue,
      parameterDefaultValue,
      normalizationPosition.minimum,
      normalizationPosition.maximum,
      normalizationPosition.defalut,
      isInverted
    ) * weight;
  }
  function getInputAngleFromNormalizedParameterValue(targetTranslation, targetAngle, value, parameterMinimumValue, parameterMaximumValue, parameterDefaultValue, normalizaitionPosition, normalizationAngle, isInverted, weight) {
    targetAngle.angle += normalizeParameterValue(
      value,
      parameterMinimumValue,
      parameterMaximumValue,
      parameterDefaultValue,
      normalizationAngle.minimum,
      normalizationAngle.maximum,
      normalizationAngle.defalut,
      isInverted
    ) * weight;
  }
  function getOutputTranslationX(translation, particles, particleIndex, isInverted, parentGravity) {
    let outputValue = translation.x;
    if (isInverted) {
      outputValue *= -1;
    }
    return outputValue;
  }
  function getOutputTranslationY(translation, particles, particleIndex, isInverted, parentGravity) {
    let outputValue = translation.y;
    if (isInverted) {
      outputValue *= -1;
    }
    return outputValue;
  }
  function getOutputAngle(translation, particles, particleIndex, isInverted, parentGravity) {
    let outputValue;
    if (particleIndex >= 2) {
      parentGravity = particles[particleIndex - 1].position.substract(
        particles[particleIndex - 2].position
      );
    } else {
      parentGravity = parentGravity.multiplyByScaler(-1);
    }
    outputValue = CubismMath.directionToRadian(parentGravity, translation);
    if (isInverted) {
      outputValue *= -1;
    }
    return outputValue;
  }
  function getRangeValue(min, max) {
    const maxValue = CubismMath.max(min, max);
    const minValue = CubismMath.min(min, max);
    return CubismMath.abs(maxValue - minValue);
  }
  function getDefaultValue(min, max) {
    const minValue = CubismMath.min(min, max);
    return minValue + getRangeValue(min, max) / 2;
  }
  function getOutputScaleTranslationX(translationScale, angleScale) {
    return JSON.parse(JSON.stringify(translationScale.x));
  }
  function getOutputScaleTranslationY(translationScale, angleScale) {
    return JSON.parse(JSON.stringify(translationScale.y));
  }
  function getOutputScaleAngle(translationScale, angleScale) {
    return JSON.parse(JSON.stringify(angleScale));
  }
  function updateParticles(strand, strandCount, totalTranslation, totalAngle, windDirection, thresholdValue, deltaTimeSeconds, airResistance) {
    let delay;
    let radian;
    let direction = new CubismVector2(0, 0);
    let velocity = new CubismVector2(0, 0);
    let force = new CubismVector2(0, 0);
    let newDirection = new CubismVector2(0, 0);
    strand[0].position = new CubismVector2(
      totalTranslation.x,
      totalTranslation.y
    );
    const totalRadian = CubismMath.degreesToRadian(totalAngle);
    const currentGravity = CubismMath.radianToDirection(totalRadian);
    currentGravity.normalize();
    for (let i = 1; i < strandCount; ++i) {
      strand[i].force = currentGravity.multiplyByScaler(strand[i].acceleration).add(windDirection);
      strand[i].lastPosition = new CubismVector2(
        strand[i].position.x,
        strand[i].position.y
      );
      delay = strand[i].delay * deltaTimeSeconds * 30;
      direction = strand[i].position.substract(strand[i - 1].position);
      radian = CubismMath.directionToRadian(strand[i].lastGravity, currentGravity) / airResistance;
      direction.x = CubismMath.cos(radian) * direction.x - direction.y * CubismMath.sin(radian);
      direction.y = CubismMath.sin(radian) * direction.x + direction.y * CubismMath.cos(radian);
      strand[i].position = strand[i - 1].position.add(direction);
      velocity = strand[i].velocity.multiplyByScaler(delay);
      force = strand[i].force.multiplyByScaler(delay).multiplyByScaler(delay);
      strand[i].position = strand[i].position.add(velocity).add(force);
      newDirection = strand[i].position.substract(strand[i - 1].position);
      newDirection.normalize();
      strand[i].position = strand[i - 1].position.add(
        newDirection.multiplyByScaler(strand[i].radius)
      );
      if (CubismMath.abs(strand[i].position.x) < thresholdValue) {
        strand[i].position.x = 0;
      }
      if (delay != 0) {
        strand[i].velocity = strand[i].position.substract(strand[i].lastPosition);
        strand[i].velocity = strand[i].velocity.divisionByScalar(delay);
        strand[i].velocity = strand[i].velocity.multiplyByScaler(
          strand[i].mobility
        );
      }
      strand[i].force = new CubismVector2(0, 0);
      strand[i].lastGravity = new CubismVector2(
        currentGravity.x,
        currentGravity.y
      );
    }
  }
  function updateParticlesForStabilization(strand, strandCount, totalTranslation, totalAngle, windDirection, thresholdValue) {
    let force = new CubismVector2(0, 0);
    strand[0].position = new CubismVector2(
      totalTranslation.x,
      totalTranslation.y
    );
    const totalRadian = CubismMath.degreesToRadian(totalAngle);
    const currentGravity = CubismMath.radianToDirection(totalRadian);
    currentGravity.normalize();
    for (let i = 1; i < strandCount; ++i) {
      strand[i].force = currentGravity.multiplyByScaler(strand[i].acceleration).add(windDirection);
      strand[i].lastPosition = new CubismVector2(
        strand[i].position.x,
        strand[i].position.y
      );
      strand[i].velocity = new CubismVector2(0, 0);
      force = strand[i].force;
      force.normalize();
      force = force.multiplyByScaler(strand[i].radius);
      strand[i].position = strand[i - 1].position.add(force);
      if (CubismMath.abs(strand[i].position.x) < thresholdValue) {
        strand[i].position.x = 0;
      }
      strand[i].force = new CubismVector2(0, 0);
      strand[i].lastGravity = new CubismVector2(
        currentGravity.x,
        currentGravity.y
      );
    }
  }
  function updateOutputParameterValue(parameterValue, parameterValueMinimum, parameterValueMaximum, translation, output) {
    let value;
    const outputScale = output.getScale(
      output.translationScale,
      output.angleScale
    );
    value = translation * outputScale;
    if (value < parameterValueMinimum) {
      if (value < output.valueBelowMinimum) {
        output.valueBelowMinimum = value;
      }
      value = parameterValueMinimum;
    } else if (value > parameterValueMaximum) {
      if (value > output.valueExceededMaximum) {
        output.valueExceededMaximum = value;
      }
      value = parameterValueMaximum;
    }
    const weight = output.weight / MaximumWeight;
    if (weight >= 1) {
      parameterValue[0] = value;
    } else {
      value = parameterValue[0] * (1 - weight) + value * weight;
      parameterValue[0] = value;
    }
  }
  function normalizeParameterValue(value, parameterMinimum, parameterMaximum, parameterDefault, normalizedMinimum, normalizedMaximum, normalizedDefault, isInverted) {
    let result = 0;
    const maxValue = CubismMath.max(parameterMaximum, parameterMinimum);
    if (maxValue < value) {
      value = maxValue;
    }
    const minValue = CubismMath.min(parameterMaximum, parameterMinimum);
    if (minValue > value) {
      value = minValue;
    }
    const minNormValue = CubismMath.min(
      normalizedMinimum,
      normalizedMaximum
    );
    const maxNormValue = CubismMath.max(
      normalizedMinimum,
      normalizedMaximum
    );
    const middleNormValue = normalizedDefault;
    const middleValue = getDefaultValue(minValue, maxValue);
    const paramValue = value - middleValue;
    switch (sign(paramValue)) {
      case 1: {
        const nLength = maxNormValue - middleNormValue;
        const pLength = maxValue - middleValue;
        if (pLength != 0) {
          result = paramValue * (nLength / pLength);
          result += middleNormValue;
        }
        break;
      }
      case -1: {
        const nLength = minNormValue - middleNormValue;
        const pLength = minValue - middleValue;
        if (pLength != 0) {
          result = paramValue * (nLength / pLength);
          result += middleNormValue;
        }
        break;
      }
      case 0: {
        result = middleNormValue;
        break;
      }
      default: {
        break;
      }
    }
    return isInverted ? result : result * -1;
  }
  var Live2DCubismFramework32;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismPhysics = CubismPhysics;
    Live2DCubismFramework51.Options = Options;
  })(Live2DCubismFramework32 || (Live2DCubismFramework32 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/model/cubismmodelmultiplyandscreencolor.ts
  var ColorData = class {
    constructor(isOverridden = false, color = new CubismTextureColor()) {
      this.isOverridden = isOverridden;
      this.color = color;
    }
  };
  var CubismModelMultiplyAndScreenColor = class {
    /**
     * Constructor.
     *
     * @param model cubism model.
     */
    constructor(model) {
      this._model = model;
      this._isOverriddenModelMultiplyColors = false;
      this._isOverriddenModelScreenColors = false;
      this._userPartScreenColors = [];
      this._userPartMultiplyColors = [];
      this._userDrawableScreenColors = [];
      this._userDrawableMultiplyColors = [];
      this._userOffscreenScreenColors = [];
      this._userOffscreenMultiplyColors = [];
    }
    /**
     * Initialization for using multiply and screen colors.
     *
     * @param partCount number of parts.
     * @param drawableCount number of drawables.
     * @param offscreenCount number of offscreen.
     */
    initialize(partCount, drawableCount, offscreenCount) {
      const userMultiplyColor = new ColorData(
        false,
        new CubismTextureColor(1, 1, 1, 1)
      );
      const userScreenColor = new ColorData(
        false,
        new CubismTextureColor(0, 0, 0, 1)
      );
      this._userPartMultiplyColors = new Array(partCount);
      this._userPartScreenColors = new Array(partCount);
      for (let i = 0; i < partCount; i++) {
        this._userPartMultiplyColors[i] = new ColorData(
          userMultiplyColor.isOverridden,
          new CubismTextureColor(
            userMultiplyColor.color.r,
            userMultiplyColor.color.g,
            userMultiplyColor.color.b,
            userMultiplyColor.color.a
          )
        );
        this._userPartScreenColors[i] = new ColorData(
          userScreenColor.isOverridden,
          new CubismTextureColor(
            userScreenColor.color.r,
            userScreenColor.color.g,
            userScreenColor.color.b,
            userScreenColor.color.a
          )
        );
      }
      this._userDrawableMultiplyColors = new Array(drawableCount);
      this._userDrawableScreenColors = new Array(drawableCount);
      for (let i = 0; i < drawableCount; i++) {
        this._userDrawableMultiplyColors[i] = new ColorData(
          userMultiplyColor.isOverridden,
          new CubismTextureColor(
            userMultiplyColor.color.r,
            userMultiplyColor.color.g,
            userMultiplyColor.color.b,
            userMultiplyColor.color.a
          )
        );
        this._userDrawableScreenColors[i] = new ColorData(
          userScreenColor.isOverridden,
          new CubismTextureColor(
            userScreenColor.color.r,
            userScreenColor.color.g,
            userScreenColor.color.b,
            userScreenColor.color.a
          )
        );
      }
      this._userOffscreenMultiplyColors = new Array(offscreenCount);
      this._userOffscreenScreenColors = new Array(offscreenCount);
      for (let i = 0; i < offscreenCount; i++) {
        this._userOffscreenMultiplyColors[i] = new ColorData(
          userMultiplyColor.isOverridden,
          new CubismTextureColor(
            userMultiplyColor.color.r,
            userMultiplyColor.color.g,
            userMultiplyColor.color.b,
            userMultiplyColor.color.a
          )
        );
        this._userOffscreenScreenColors[i] = new ColorData(
          userScreenColor.isOverridden,
          new CubismTextureColor(
            userScreenColor.color.r,
            userScreenColor.color.g,
            userScreenColor.color.b,
            userScreenColor.color.a
          )
        );
      }
    }
    /**
     * Outputs a warning message for index out of range errors.
     *
     * @param functionName Name of the calling function
     * @param index The invalid index value
     * @param maxIndex The maximum valid index (length - 1)
     */
    warnIndexOutOfRange(functionName, index, maxIndex) {
      CubismLogWarning(
        `${functionName}: index is out of range. index=${index}, valid range=[0, ${maxIndex}].`
      );
    }
    /**
     * Validates if the given part index is within valid range.
     *
     * @param index Part index to validate
     * @param functionName Name of the calling function for error reporting
     * @return true if the index is valid; otherwise false
     */
    isValidPartIndex(index, functionName) {
      if (index < 0 || index >= this._model.getPartCount()) {
        this.warnIndexOutOfRange(
          functionName,
          index,
          this._model.getPartCount() - 1
        );
        return false;
      }
      return true;
    }
    /**
     * Validates if the given drawable index is within valid range.
     *
     * @param index Drawable index to validate
     * @param functionName Name of the calling function for error reporting
     * @return true if the index is valid; otherwise false
     */
    isValidDrawableIndex(index, functionName) {
      if (index < 0 || index >= this._model.getDrawableCount()) {
        this.warnIndexOutOfRange(
          functionName,
          index,
          this._model.getDrawableCount() - 1
        );
        return false;
      }
      return true;
    }
    /**
     * Validates if the given offscreen index is within valid range.
     *
     * @param index Offscreen index to validate
     * @param functionName Name of the calling function for error reporting
     * @return true if the index is valid; otherwise false
     */
    isValidOffscreenIndex(index, functionName) {
      if (index < 0 || index >= this._model.getOffscreenCount()) {
        this.warnIndexOutOfRange(
          functionName,
          index,
          this._model.getOffscreenCount() - 1
        );
        return false;
      }
      return true;
    }
    /**
     * Sets the flag indicating whether the color set at runtime is used as the multiply color for the entire model during rendering.
     *
     * @param value true if the color set at runtime is to be used; otherwise false.
     */
    setMultiplyColorEnabled(value) {
      this._isOverriddenModelMultiplyColors = value;
    }
    /**
     * Returns the flag indicating whether the color set at runtime is used as the multiply color for the entire model during rendering.
     *
     * @return true if the color set at runtime is used; otherwise false.
     */
    getMultiplyColorEnabled() {
      return this._isOverriddenModelMultiplyColors;
    }
    /**
     * Sets the flag indicating whether the color set at runtime is used as the screen color for the entire model during rendering.
     *
     * @param value true if the color set at runtime is to be used; otherwise false.
     */
    setScreenColorEnabled(value) {
      this._isOverriddenModelScreenColors = value;
    }
    /**
     * Returns the flag indicating whether the color set at runtime is used as the screen color for the entire model during rendering.
     *
     * @return true if the color set at runtime is used; otherwise false.
     */
    getScreenColorEnabled() {
      return this._isOverriddenModelScreenColors;
    }
    /**
     * Sets whether the part multiply color is overridden by the SDK.
     * Use true to use the color information from the SDK, or false to use the color information from the model.
     *
     * @param partIndex Part index
     * @param value true enable override, false to disable
     */
    setPartMultiplyColorEnabled(partIndex, value) {
      if (!this.isValidPartIndex(partIndex, "setPartMultiplyColorEnabled")) {
        return;
      }
      this.setPartColorEnabled(
        partIndex,
        value,
        this._userPartMultiplyColors,
        this._userDrawableMultiplyColors,
        this._userOffscreenMultiplyColors
      );
    }
    /**
     * Checks whether the part multiply color is overridden by the SDK.
     *
     * @param partIndex Part index
     *
     * @return true if the color information from the SDK is used; otherwise false.
     */
    getPartMultiplyColorEnabled(partIndex) {
      if (!this.isValidPartIndex(partIndex, "getPartMultiplyColorEnabled")) {
        return false;
      }
      return this._userPartMultiplyColors[partIndex].isOverridden;
    }
    /**
     * Sets whether the part screen color is overridden by the SDK.
     * Use true to use the color information from the SDK, or false to use the color information from the model.
     *
     * @param partIndex Part index
     * @param value true enable override, false to disable
     */
    setPartScreenColorEnabled(partIndex, value) {
      if (!this.isValidPartIndex(partIndex, "setPartScreenColorEnabled")) {
        return;
      }
      this.setPartColorEnabled(
        partIndex,
        value,
        this._userPartScreenColors,
        this._userDrawableScreenColors,
        this._userOffscreenScreenColors
      );
    }
    /**
     * Checks whether the part screen color is overridden by the SDK.
     *
     * @param partIndex Part index
     *
     * @return true if the color information from the SDK is used; otherwise false.
     */
    getPartScreenColorEnabled(partIndex) {
      if (!this.isValidPartIndex(partIndex, "getPartScreenColorEnabled")) {
        return false;
      }
      return this._userPartScreenColors[partIndex].isOverridden;
    }
    /**
     * Sets the multiply color of the part.
     *
     * @param partIndex Part index
     * @param color Multiply color to be set (CubismTextureColor)
     */
    setPartMultiplyColorByTextureColor(partIndex, color) {
      if (!this.isValidPartIndex(partIndex, "setPartMultiplyColorByTextureColor")) {
        return;
      }
      this.setPartMultiplyColorByRGBA(
        partIndex,
        color.r,
        color.g,
        color.b,
        color.a
      );
    }
    /**
     * Sets the multiply color of the part.
     *
     * @param partIndex Part index
     * @param r Red value of the multiply color to be set
     * @param g Green value of the multiply color to be set
     * @param b Blue value of the multiply color to be set
     * @param a Alpha value of the multiply color to be set
     */
    setPartMultiplyColorByRGBA(partIndex, r, g, b, a = 1) {
      if (!this.isValidPartIndex(partIndex, "setPartMultiplyColorByRGBA")) {
        return;
      }
      this.setPartColor(
        partIndex,
        r,
        g,
        b,
        a,
        this._userPartMultiplyColors,
        this._userDrawableMultiplyColors,
        this._userOffscreenMultiplyColors
      );
    }
    /**
     * Returns the multiply color of the part.
     *
     * @param partIndex Part index
     *
     * @return Multiply color (CubismTextureColor)
     */
    getPartMultiplyColor(partIndex) {
      if (!this.isValidPartIndex(partIndex, "getPartMultiplyColor")) {
        return new CubismTextureColor(1, 1, 1, 1);
      }
      return this._userPartMultiplyColors[partIndex].color;
    }
    /**
     * Sets the screen color of the part.
     *
     * @param partIndex Part index
     * @param color Screen color to be set (CubismTextureColor)
     */
    setPartScreenColorByTextureColor(partIndex, color) {
      if (!this.isValidPartIndex(partIndex, "setPartScreenColorByTextureColor")) {
        return;
      }
      this.setPartScreenColorByRGBA(
        partIndex,
        color.r,
        color.g,
        color.b,
        color.a
      );
    }
    /**
     * Sets the screen color of the part.
     *
     * @param partIndex Part index
     * @param r Red value of the screen color to be set
     * @param g Green value of the screen color to be set
     * @param b Blue value of the screen color to be set
     * @param a Alpha value of the screen color to be set
     */
    setPartScreenColorByRGBA(partIndex, r, g, b, a = 1) {
      if (!this.isValidPartIndex(partIndex, "setPartScreenColorByRGBA")) {
        return;
      }
      this.setPartColor(
        partIndex,
        r,
        g,
        b,
        a,
        this._userPartScreenColors,
        this._userDrawableScreenColors,
        this._userOffscreenScreenColors
      );
    }
    /**
     * Returns the screen color of the part.
     *
     * @param partIndex Part index
     *
     * @return Screen color (CubismTextureColor)
     */
    getPartScreenColor(partIndex) {
      if (!this.isValidPartIndex(partIndex, "getPartScreenColor")) {
        return new CubismTextureColor(0, 0, 0, 1);
      }
      return this._userPartScreenColors[partIndex].color;
    }
    /**
     * Sets the flag indicating whether the color set at runtime is used as the multiply color for the drawable during rendering.
     *
     * @param drawableIndex Drawable index
     * @param value true if the color set at runtime is to be used; otherwise false.
     */
    setDrawableMultiplyColorEnabled(drawableIndex, value) {
      if (!this.isValidDrawableIndex(
        drawableIndex,
        "setDrawableMultiplyColorEnabled"
      )) {
        return;
      }
      this._userDrawableMultiplyColors[drawableIndex].isOverridden = value;
    }
    /**
     * Returns the flag indicating whether the color set at runtime is used as the multiply color for the drawable during rendering.
     *
     * @param drawableIndex Drawable index
     *
     * @return true if the color set at runtime is used; otherwise false.
     */
    getDrawableMultiplyColorEnabled(drawableIndex) {
      if (!this.isValidDrawableIndex(
        drawableIndex,
        "getDrawableMultiplyColorEnabled"
      )) {
        return false;
      }
      return this._userDrawableMultiplyColors[drawableIndex].isOverridden;
    }
    /**
     * Sets the flag indicating whether the color set at runtime is used as the screen color for the drawable during rendering.
     *
     * @param drawableIndex Drawable index
     * @param value true if the color set at runtime is to be used; otherwise false.
     */
    setDrawableScreenColorEnabled(drawableIndex, value) {
      if (!this.isValidDrawableIndex(drawableIndex, "setDrawableScreenColorEnabled")) {
        return;
      }
      this._userDrawableScreenColors[drawableIndex].isOverridden = value;
    }
    /**
     * Returns the flag indicating whether the color set at runtime is used as the screen color for the drawable during rendering.
     *
     * @param drawableIndex Drawable index
     *
     * @return true if the color set at runtime is used; otherwise false.
     */
    getDrawableScreenColorEnabled(drawableIndex) {
      if (!this.isValidDrawableIndex(drawableIndex, "getDrawableScreenColorEnabled")) {
        return false;
      }
      return this._userDrawableScreenColors[drawableIndex].isOverridden;
    }
    /**
     * Sets the multiply color of the drawable.
     *
     * @param drawableIndex Drawable index
     * @param color Multiply color to be set (CubismTextureColor)
     */
    setDrawableMultiplyColorByTextureColor(drawableIndex, color) {
      if (!this.isValidDrawableIndex(
        drawableIndex,
        "setDrawableMultiplyColorByTextureColor"
      )) {
        return;
      }
      this.setDrawableMultiplyColorByRGBA(
        drawableIndex,
        color.r,
        color.g,
        color.b,
        color.a
      );
    }
    /**
     * Sets the multiply color of the drawable.
     *
     * @param drawableIndex Drawable index
     * @param r Red value of the multiply color to be set
     * @param g Green value of the multiply color to be set
     * @param b Blue value of the multiply color to be set
     * @param a Alpha value of the multiply color to be set
     */
    setDrawableMultiplyColorByRGBA(drawableIndex, r, g, b, a = 1) {
      if (!this.isValidDrawableIndex(
        drawableIndex,
        "setDrawableMultiplyColorByRGBA"
      )) {
        return;
      }
      this._userDrawableMultiplyColors[drawableIndex].color.r = r;
      this._userDrawableMultiplyColors[drawableIndex].color.g = g;
      this._userDrawableMultiplyColors[drawableIndex].color.b = b;
      this._userDrawableMultiplyColors[drawableIndex].color.a = a;
    }
    /**
     * Returns the multiply color from the list of drawables.
     *
     * @param drawableIndex Drawable index
     *
     * @return Multiply color (CubismTextureColor)
     */
    getDrawableMultiplyColor(drawableIndex) {
      if (!this.isValidDrawableIndex(drawableIndex, "getDrawableMultiplyColor")) {
        return new CubismTextureColor(1, 1, 1, 1);
      }
      if (this.getMultiplyColorEnabled() || this.getDrawableMultiplyColorEnabled(drawableIndex)) {
        return this._userDrawableMultiplyColors[drawableIndex].color;
      }
      return this._model.getDrawableMultiplyColor(drawableIndex);
    }
    /**
     * Sets the screen color of the drawable.
     *
     * @param drawableIndex Drawable index
     * @param color Screen color to be set (CubismTextureColor)
     */
    setDrawableScreenColorByTextureColor(drawableIndex, color) {
      if (!this.isValidDrawableIndex(
        drawableIndex,
        "setDrawableScreenColorByTextureColor"
      )) {
        return;
      }
      this.setDrawableScreenColorByRGBA(
        drawableIndex,
        color.r,
        color.g,
        color.b,
        color.a
      );
    }
    /**
     * Sets the screen color of the drawable.
     *
     * @param drawableIndex Drawable index
     * @param r Red value of the screen color to be set
     * @param g Green value of the screen color to be set
     * @param b Blue value of the screen color to be set
     * @param a Alpha value of the screen color to be set
     */
    setDrawableScreenColorByRGBA(drawableIndex, r, g, b, a = 1) {
      if (!this.isValidDrawableIndex(drawableIndex, "setDrawableScreenColorByRGBA")) {
        return;
      }
      this._userDrawableScreenColors[drawableIndex].color.r = r;
      this._userDrawableScreenColors[drawableIndex].color.g = g;
      this._userDrawableScreenColors[drawableIndex].color.b = b;
      this._userDrawableScreenColors[drawableIndex].color.a = a;
    }
    /**
     * Returns the screen color from the list of drawables.
     *
     * @param drawableIndex Drawable index
     *
     * @return Screen color (CubismTextureColor)
     */
    getDrawableScreenColor(drawableIndex) {
      if (!this.isValidDrawableIndex(drawableIndex, "getDrawableScreenColor")) {
        return new CubismTextureColor(0, 0, 0, 1);
      }
      if (this.getScreenColorEnabled() || this.getDrawableScreenColorEnabled(drawableIndex)) {
        return this._userDrawableScreenColors[drawableIndex].color;
      }
      return this._model.getDrawableScreenColor(drawableIndex);
    }
    /**
     * Sets whether the offscreen multiply color is overridden by the SDK.
     * Use true to use the color information from the SDK, or false to use the color information from the model.
     *
     * @param offscreenIndex Offscreen index
     * @param value true enable override, false to disable
     */
    setOffscreenMultiplyColorEnabled(offscreenIndex, value) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "setOffscreenMultiplyColorEnabled"
      )) {
        return;
      }
      this._userOffscreenMultiplyColors[offscreenIndex].isOverridden = value;
    }
    /**
     * Checks whether the offscreen multiply color is overridden by the SDK.
     *
     * @param offscreenIndex Offscreen index
     *
     * @return true if the color information from the SDK is used; otherwise false.
     */
    getOffscreenMultiplyColorEnabled(offscreenIndex) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "getOffscreenMultiplyColorEnabled"
      )) {
        return false;
      }
      return this._userOffscreenMultiplyColors[offscreenIndex].isOverridden;
    }
    /**
     * Sets whether the offscreen screen color is overridden by the SDK.
     * Use true to use the color information from the SDK, or false to use the color information from the model.
     *
     * @param offscreenIndex Offscreen index
     * @param value true enable override, false to disable
     */
    setOffscreenScreenColorEnabled(offscreenIndex, value) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "setOffscreenScreenColorEnabled"
      )) {
        return;
      }
      this._userOffscreenScreenColors[offscreenIndex].isOverridden = value;
    }
    /**
     * Checks whether the offscreen screen color is overridden by the SDK.
     *
     * @param offscreenIndex Offscreen index
     *
     * @return true if the color information from the SDK is used; otherwise false.
     */
    getOffscreenScreenColorEnabled(offscreenIndex) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "getOffscreenScreenColorEnabled"
      )) {
        return false;
      }
      return this._userOffscreenScreenColors[offscreenIndex].isOverridden;
    }
    /**
     * Sets the multiply color of the offscreen.
     *
     * @param offscreenIndex Offsscreen index
     * @param color Multiply color to be set (CubismTextureColor)
     */
    setOffscreenMultiplyColorByTextureColor(offscreenIndex, color) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "setOffscreenMultiplyColorByTextureColor"
      )) {
        return;
      }
      this.setOffscreenMultiplyColorByRGBA(
        offscreenIndex,
        color.r,
        color.g,
        color.b,
        color.a
      );
    }
    /**
     * Sets the multiply color of the offscreen.
     *
     * @param offscreenIndex Offsscreen index
     * @param r Red value of the multiply color to be set
     * @param g Green value of the multiply color to be set
     * @param b Blue value of the multiply color to be set
     * @param a Alpha value of the multiply color to be set
     */
    setOffscreenMultiplyColorByRGBA(offscreenIndex, r, g, b, a = 1) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "setOffscreenMultiplyColorByRGBA"
      )) {
        return;
      }
      this._userOffscreenMultiplyColors[offscreenIndex].color.r = r;
      this._userOffscreenMultiplyColors[offscreenIndex].color.g = g;
      this._userOffscreenMultiplyColors[offscreenIndex].color.b = b;
      this._userOffscreenMultiplyColors[offscreenIndex].color.a = a;
    }
    /**
     * Returns the multiply color from the list of offscreen.
     *
     * @param offscreenIndex Offsscreen index
     *
     * @return Multiply color (CubismTextureColor)
     */
    getOffscreenMultiplyColor(offscreenIndex) {
      if (!this.isValidOffscreenIndex(offscreenIndex, "getOffscreenMultiplyColor")) {
        return new CubismTextureColor(1, 1, 1, 1);
      }
      if (this.getMultiplyColorEnabled() || this.getOffscreenMultiplyColorEnabled(offscreenIndex)) {
        return this._userOffscreenMultiplyColors[offscreenIndex].color;
      }
      return this._model.getOffscreenMultiplyColor(offscreenIndex);
    }
    /**
     * Sets the screen color of the offscreen.
     *
     * @param offscreenIndex Offsscreen index
     * @param color Screen color to be set (CubismTextureColor)
     */
    setOffscreenScreenColorByTextureColor(offscreenIndex, color) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "setOffscreenScreenColorByTextureColor"
      )) {
        return;
      }
      this.setOffscreenScreenColorByRGBA(
        offscreenIndex,
        color.r,
        color.g,
        color.b,
        color.a
      );
    }
    /**
     * Sets the screen color of the offscreen.
     *
     * @param offscreenIndex Offsscreen index
     * @param r Red value of the screen color to be set
     * @param g Green value of the screen color to be set
     * @param b Blue value of the screen color to be set
     * @param a Alpha value of the screen color to be set
     */
    setOffscreenScreenColorByRGBA(offscreenIndex, r, g, b, a = 1) {
      if (!this.isValidOffscreenIndex(
        offscreenIndex,
        "setOffscreenScreenColorByRGBA"
      )) {
        return;
      }
      this._userOffscreenScreenColors[offscreenIndex].color.r = r;
      this._userOffscreenScreenColors[offscreenIndex].color.g = g;
      this._userOffscreenScreenColors[offscreenIndex].color.b = b;
      this._userOffscreenScreenColors[offscreenIndex].color.a = a;
    }
    /**
     * Returns the screen color from the list of offscreen.
     *
     * @param offscreenIndex Offsscreen index
     *
     * @return Screen color (CubismTextureColor)
     */
    getOffscreenScreenColor(offscreenIndex) {
      if (!this.isValidOffscreenIndex(offscreenIndex, "getOffscreenScreenColor")) {
        return new CubismTextureColor(0, 0, 0, 1);
      }
      if (this.getScreenColorEnabled() || this.getOffscreenScreenColorEnabled(offscreenIndex)) {
        return this._userOffscreenScreenColors[offscreenIndex].color;
      }
      return this._model.getOffscreenScreenColor(offscreenIndex);
    }
    /**
     * Sets the part color with hierarchical propagation (internal method)
     */
    setPartColor(partIndex, r, g, b, a, partColors, drawableColors, offscreenColors) {
      partColors[partIndex].color.r = r;
      partColors[partIndex].color.g = g;
      partColors[partIndex].color.b = b;
      partColors[partIndex].color.a = a;
      if (partColors[partIndex].isOverridden) {
        const offscreenIndices = this._model.getPartOffscreenIndices();
        const offscreenIndex = offscreenIndices[partIndex];
        if (offscreenIndex == NoOffscreenIndex) {
          const partsHierarchy = this._model.getPartsHierarchy();
          if (partsHierarchy && partsHierarchy[partIndex]) {
            for (let i = 0; i < partsHierarchy[partIndex].objects.length; ++i) {
              const objectInfo = partsHierarchy[partIndex].objects[i];
              if (objectInfo.objectType === 0 /* CubismModelObjectType_Drawable */) {
                const drawableIndex = objectInfo.objectIndex;
                drawableColors[drawableIndex].color.r = r;
                drawableColors[drawableIndex].color.g = g;
                drawableColors[drawableIndex].color.b = b;
                drawableColors[drawableIndex].color.a = a;
              } else {
                const childPartIndex = objectInfo.objectIndex;
                this.setPartColor(
                  childPartIndex,
                  r,
                  g,
                  b,
                  a,
                  partColors,
                  drawableColors,
                  offscreenColors
                );
              }
            }
          }
        } else {
          offscreenColors[offscreenIndex].color.r = r;
          offscreenColors[offscreenIndex].color.g = g;
          offscreenColors[offscreenIndex].color.b = b;
          offscreenColors[offscreenIndex].color.a = a;
        }
      }
    }
    /**
     * Sets the part color enabled flag with hierarchical propagation (internal method)
     */
    setPartColorEnabled(partIndex, value, partColors, drawableColors, offscreenColors) {
      partColors[partIndex].isOverridden = value;
      const offscreenIndices = this._model.getPartOffscreenIndices();
      const offscreenIndex = offscreenIndices[partIndex];
      if (offscreenIndex == NoOffscreenIndex) {
        const partsHierarchy = this._model.getPartsHierarchy();
        if (partsHierarchy && partsHierarchy[partIndex]) {
          for (let i = 0; i < partsHierarchy[partIndex].objects.length; ++i) {
            const objectInfo = partsHierarchy[partIndex].objects[i];
            if (objectInfo.objectType === 0 /* CubismModelObjectType_Drawable */) {
              const drawableIndex = objectInfo.objectIndex;
              drawableColors[drawableIndex].isOverridden = value;
              if (value) {
                drawableColors[drawableIndex].color.r = partColors[partIndex].color.r;
                drawableColors[drawableIndex].color.g = partColors[partIndex].color.g;
                drawableColors[drawableIndex].color.b = partColors[partIndex].color.b;
                drawableColors[drawableIndex].color.a = partColors[partIndex].color.a;
              }
            } else {
              const childPartIndex = objectInfo.objectIndex;
              if (value) {
                partColors[childPartIndex].color.r = partColors[partIndex].color.r;
                partColors[childPartIndex].color.g = partColors[partIndex].color.g;
                partColors[childPartIndex].color.b = partColors[partIndex].color.b;
                partColors[childPartIndex].color.a = partColors[partIndex].color.a;
              }
              this.setPartColorEnabled(
                childPartIndex,
                value,
                partColors,
                drawableColors,
                offscreenColors
              );
            }
          }
        }
      } else {
        offscreenColors[offscreenIndex].isOverridden = value;
        if (value) {
          offscreenColors[offscreenIndex].color.r = partColors[partIndex].color.r;
          offscreenColors[offscreenIndex].color.g = partColors[partIndex].color.g;
          offscreenColors[offscreenIndex].color.b = partColors[partIndex].color.b;
          offscreenColors[offscreenIndex].color.a = partColors[partIndex].color.a;
        }
      }
    }
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/model/cubismmodel.ts
  var NoParentIndex = -1;
  var NoOffscreenIndex = -1;
  var CubismColorBlend = ((CubismColorBlend2) => {
    CubismColorBlend2[CubismColorBlend2["ColorBlend_None"] = -1] = "ColorBlend_None";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Normal"] = Live2DCubismCore.ColorBlendType_Normal] = "ColorBlend_Normal";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_AddGlow"] = Live2DCubismCore.ColorBlendType_AddGlow] = "ColorBlend_AddGlow";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Add"] = Live2DCubismCore.ColorBlendType_Add] = "ColorBlend_Add";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Darken"] = Live2DCubismCore.ColorBlendType_Darken] = "ColorBlend_Darken";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Multiply"] = Live2DCubismCore.ColorBlendType_Multiply] = "ColorBlend_Multiply";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_ColorBurn"] = Live2DCubismCore.ColorBlendType_ColorBurn] = "ColorBlend_ColorBurn";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_LinearBurn"] = Live2DCubismCore.ColorBlendType_LinearBurn] = "ColorBlend_LinearBurn";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Lighten"] = Live2DCubismCore.ColorBlendType_Lighten] = "ColorBlend_Lighten";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Screen"] = Live2DCubismCore.ColorBlendType_Screen] = "ColorBlend_Screen";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_ColorDodge"] = Live2DCubismCore.ColorBlendType_ColorDodge] = "ColorBlend_ColorDodge";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Overlay"] = Live2DCubismCore.ColorBlendType_Overlay] = "ColorBlend_Overlay";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_SoftLight"] = Live2DCubismCore.ColorBlendType_SoftLight] = "ColorBlend_SoftLight";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_HardLight"] = Live2DCubismCore.ColorBlendType_HardLight] = "ColorBlend_HardLight";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_LinearLight"] = Live2DCubismCore.ColorBlendType_LinearLight] = "ColorBlend_LinearLight";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Hue"] = Live2DCubismCore.ColorBlendType_Hue] = "ColorBlend_Hue";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_Color"] = Live2DCubismCore.ColorBlendType_Color] = "ColorBlend_Color";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_AddCompatible"] = Live2DCubismCore.ColorBlendType_AddCompatible] = "ColorBlend_AddCompatible";
    CubismColorBlend2[CubismColorBlend2["ColorBlend_MultiplyCompatible"] = Live2DCubismCore.ColorBlendType_MultiplyCompatible] = "ColorBlend_MultiplyCompatible";
    return CubismColorBlend2;
  })(CubismColorBlend || {});
  var CubismAlphaBlend = /* @__PURE__ */ ((CubismAlphaBlend2) => {
    CubismAlphaBlend2[CubismAlphaBlend2["AlphaBlend_None"] = -1] = "AlphaBlend_None";
    CubismAlphaBlend2[CubismAlphaBlend2["AlphaBlend_Over"] = 0] = "AlphaBlend_Over";
    CubismAlphaBlend2[CubismAlphaBlend2["AlphaBlend_Atop"] = 1] = "AlphaBlend_Atop";
    CubismAlphaBlend2[CubismAlphaBlend2["AlphaBlend_Out"] = 2] = "AlphaBlend_Out";
    CubismAlphaBlend2[CubismAlphaBlend2["AlphaBlend_ConjointOver"] = 3] = "AlphaBlend_ConjointOver";
    CubismAlphaBlend2[CubismAlphaBlend2["AlphaBlend_DisjointOver"] = 4] = "AlphaBlend_DisjointOver";
    return CubismAlphaBlend2;
  })(CubismAlphaBlend || {});
  var ParameterRepeatData = class {
    /**
     * Constructor
     *
     * @param isOverridden whether to be overriden
     * @param isParameterRepeated override flag for settings
     */
    constructor(isOverridden = false, isParameterRepeated = false) {
      this.isOverridden = isOverridden;
      this.isParameterRepeated = isParameterRepeated;
    }
  };
  var CullingData = class {
    /**
     * コンストラクタ
     *
     * @param isOverridden
     * @param isCulling
     */
    constructor(isOverridden = false, isCulling = false) {
      this.isOverridden = isOverridden;
      this.isCulling = isCulling;
    }
  };
  var PartChildDrawObjects = class {
    constructor(drawableIndices = new Array(), offscreenIndices = new Array()) {
      this.drawableIndices = drawableIndices;
      this.offscreenIndices = offscreenIndices;
    }
  };
  var CubismModelObjectInfo = class {
    // オブジェクトインデックス
    constructor(objectIndex, objectType) {
      this.objectIndex = objectIndex;
      this.objectType = objectType;
    }
  };
  var CubismModelPartInfo = class {
    constructor(objects = new Array(), childDrawObjects = new PartChildDrawObjects()) {
      this.objects = objects;
      this.childDrawObjects = childDrawObjects;
    }
    // 子オブジェクト数を返す関数
    getChildObjectCount() {
      return this.objects.length;
    }
  };
  var CubismModel = class {
    /**
     * モデルのパラメータの更新
     */
    update() {
      this._model.update();
      this._model.drawables.resetDynamicFlags();
    }
    /**
     * PixelsPerUnitを取得する
     * @return PixelsPerUnit
     */
    getPixelsPerUnit() {
      if (this._model == null) {
        return 0;
      }
      return this._model.canvasinfo.PixelsPerUnit;
    }
    /**
     * キャンバスの幅を取得する
     */
    getCanvasWidth() {
      if (this._model == null) {
        return 0;
      }
      return this._model.canvasinfo.CanvasWidth / this._model.canvasinfo.PixelsPerUnit;
    }
    /**
     * キャンバスの高さを取得する
     */
    getCanvasHeight() {
      if (this._model == null) {
        return 0;
      }
      return this._model.canvasinfo.CanvasHeight / this._model.canvasinfo.PixelsPerUnit;
    }
    /**
     * パラメータを保存する
     */
    saveParameters() {
      const parameterCount = this._model.parameters.count;
      const savedParameterCount = this._savedParameters.length;
      for (let i = 0; i < parameterCount; ++i) {
        if (i < savedParameterCount) {
          this._savedParameters[i] = this._parameterValues[i];
        } else {
          this._savedParameters.push(this._parameterValues[i]);
        }
      }
    }
    /**
     * 乗算色・スクリーン色管理クラスを取得する
     *
     * @return CubismModelMultiplyAndScreenColorのインスタンス
     */
    getOverrideMultiplyAndScreenColor() {
      return this._overrideMultiplyAndScreenColor;
    }
    /**
     * Checks whether parameter repetition is performed for the entire model.
     *
     * @return true if parameter repetition is performed for the entire model; otherwise returns false.
     */
    getOverrideFlagForModelParameterRepeat() {
      return this._isOverriddenParameterRepeat;
    }
    /**
     * Sets whether parameter repetition is performed for the entire model.
     * Use true to perform parameter repetition for the entire model, or false to not perform it.
     */
    setOverrideFlagForModelParameterRepeat(isRepeat) {
      this._isOverriddenParameterRepeat = isRepeat;
    }
    /**
     * Returns the flag indicating whether to override the parameter repeat.
     *
     * @param parameterIndex Parameter index
     *
     * @return true if the parameter repeat is overridden, false otherwise.
     */
    getOverrideFlagForParameterRepeat(parameterIndex) {
      return this._userParameterRepeatDataList[parameterIndex].isOverridden;
    }
    /**
     * Sets the flag indicating whether to override the parameter repeat.
     *
     * @param parameterIndex Parameter index
     * @param value true if it is to be overridden; otherwise, false.
     */
    setOverrideFlagForParameterRepeat(parameterIndex, value) {
      this._userParameterRepeatDataList[parameterIndex].isOverridden = value;
    }
    /**
     * Returns the repeat flag.
     *
     * @param parameterIndex Parameter index
     *
     * @return true if repeating, false otherwise.
     */
    getRepeatFlagForParameterRepeat(parameterIndex) {
      return this._userParameterRepeatDataList[parameterIndex].isParameterRepeated;
    }
    /**
     * Sets the repeat flag.
     *
     * @param parameterIndex Parameter index
     * @param value true to enable repeating, false otherwise.
     */
    setRepeatFlagForParameterRepeat(parameterIndex, value) {
      this._userParameterRepeatDataList[parameterIndex].isParameterRepeated = value;
    }
    /**
     * Drawableのカリング情報を取得する。
     *
     * @param   drawableIndex   Drawableのインデックス
     *
     * @return  Drawableのカリング情報
     */
    getDrawableCulling(drawableIndex) {
      if (this.getOverrideFlagForModelCullings() || this.getOverrideFlagForDrawableCullings(drawableIndex)) {
        return this._userDrawableCullings[drawableIndex].isCulling;
      }
      const constantFlags = this._model.drawables.constantFlags;
      return !Live2DCubismCore.Utils.hasIsDoubleSidedBit(
        constantFlags[drawableIndex]
      );
    }
    /**
     * Drawableのカリング情報を設定する。
     *
     * @param drawableIndex Drawableのインデックス
     * @param isCulling カリング情報
     */
    setDrawableCulling(drawableIndex, isCulling) {
      this._userDrawableCullings[drawableIndex].isCulling = isCulling;
    }
    /**
     * Offscreenのカリング情報を取得する。
     *
     * @param   offscreenIndex   Offscreenのインデックス
     *
     * @return  Offscreenのカリング情報
     */
    getOffscreenCulling(offscreenIndex) {
      if (this.getOverrideFlagForModelCullings() || this.getOverrideFlagForOffscreenCullings(offscreenIndex)) {
        return this._userOffscreenCullings[offscreenIndex].isCulling;
      }
      const constantFlags = this._model.offscreens.constantFlags;
      return !Live2DCubismCore.Utils.hasIsDoubleSidedBit(
        constantFlags[offscreenIndex]
      );
    }
    /**
     * Offscreenのカリング設定を設定する。
     *
     * @param offscreenIndex Offscreenのインデックス
     * @param isCulling カリング情報
     */
    setOffscreenCulling(offscreenIndex, isCulling) {
      this._userOffscreenCullings[offscreenIndex].isCulling = isCulling;
    }
    /**
     * SDKからモデル全体のカリング設定を上書きするか。
     *
     * @return  true    ->  SDK上のカリング設定を使用
     *          false   ->  モデルのカリング設定を使用
     */
    getOverrideFlagForModelCullings() {
      return this._isOverriddenCullings;
    }
    /**
     * SDKからモデル全体のカリング設定を上書きするかを設定する。
     *
     * @param isOverriddenCullings SDK上のカリング設定を使うならtrue、モデルのカリング設定を使うならfalse
     */
    setOverrideFlagForModelCullings(isOverriddenCullings) {
      this._isOverriddenCullings = isOverriddenCullings;
    }
    /**
     *
     * @param drawableIndex Drawableのインデックス
     * @return  true    ->  SDK上のカリング設定を使用
     *          false   ->  モデルのカリング設定を使用
     */
    getOverrideFlagForDrawableCullings(drawableIndex) {
      return this._userDrawableCullings[drawableIndex].isOverridden;
    }
    /**
     * @param offscreenIndex Offscreenのインデックス
     * @return  true    ->  SDK上のカリング設定を使用
     *          false   ->  モデルのカリング設定を使用
     */
    getOverrideFlagForOffscreenCullings(offscreenIndex) {
      return this._userOffscreenCullings[offscreenIndex].isOverridden;
    }
    /**
     *
     * @param drawableIndex Drawableのインデックス
     * @param isOverriddenCullings SDK上のカリング設定を使うならtrue、モデルのカリング設定を使うならfalse
     */
    setOverrideFlagForDrawableCullings(drawableIndex, isOverriddenCullings) {
      this._userDrawableCullings[drawableIndex].isOverridden = isOverriddenCullings;
    }
    /**
     * モデルの不透明度を取得する
     *
     * @return 不透明度の値
     */
    getModelOapcity() {
      return this._modelOpacity;
    }
    /**
     * モデルの不透明度を設定する
     *
     * @param value 不透明度の値
     */
    setModelOapcity(value) {
      this._modelOpacity = value;
    }
    /**
     * モデルを取得
     */
    getModel() {
      return this._model;
    }
    /**
     * パーツのインデックスを取得
     * @param partId パーツのID
     * @return パーツのインデックス
     */
    getPartIndex(partId) {
      let partIndex;
      const partCount = this._model.parts.count;
      for (partIndex = 0; partIndex < partCount; ++partIndex) {
        if (partId == this._partIds[partIndex]) {
          return partIndex;
        }
      }
      if (this._notExistPartId.has(partId)) {
        return this._notExistPartId.get(partId);
      }
      partIndex = partCount + this._notExistPartId.size;
      this._notExistPartId.set(partId, partIndex);
      this._notExistPartOpacities.set(partIndex, null);
      return partIndex;
    }
    /**
     * パーツのIDを取得する。
     *
     * @param partIndex 取得するパーツのインデックス
     * @return パーツのID
     */
    getPartId(partIndex) {
      const partId = this._model.parts.ids[partIndex];
      return CubismFramework.getIdManager().getId(partId);
    }
    /**
     * パーツの個数の取得
     * @return パーツの個数
     */
    getPartCount() {
      const partCount = this._model.parts.count;
      return partCount;
    }
    /**
     * パーツのオフスクリーンインデックスの取得
     * @param partIndex パーツのインデックス
     * @return オフスクリーンインデックスのリスト
     */
    getPartOffscreenIndices() {
      const offscreenIndices = this._model.parts.offscreenIndices;
      return offscreenIndices;
    }
    /**
     * パーツの親パーツインデックスのリストを取得
     *
     * @return パーツの親パーツインデックスのリスト
     */
    getPartParentPartIndices() {
      const parentIndices = this._model.parts.parentIndices;
      return parentIndices;
    }
    /**
     * パーツの不透明度の設定(Index)
     * @param partIndex パーツのインデックス
     * @param opacity 不透明度
     */
    setPartOpacityByIndex(partIndex, opacity) {
      if (this._notExistPartOpacities.has(partIndex)) {
        this._notExistPartOpacities.set(partIndex, opacity);
        return;
      }
      CSM_ASSERT(0 <= partIndex && partIndex < this.getPartCount());
      this._partOpacities[partIndex] = opacity;
    }
    /**
     * パーツの不透明度の設定(Id)
     * @param partId パーツのID
     * @param opacity パーツの不透明度
     */
    setPartOpacityById(partId, opacity) {
      const index = this.getPartIndex(partId);
      if (index < 0) {
        return;
      }
      this.setPartOpacityByIndex(index, opacity);
    }
    /**
     * パーツの不透明度の取得(index)
     * @param partIndex パーツのインデックス
     * @return パーツの不透明度
     */
    getPartOpacityByIndex(partIndex) {
      if (this._notExistPartOpacities.has(partIndex)) {
        return this._notExistPartOpacities.get(partIndex);
      }
      CSM_ASSERT(0 <= partIndex && partIndex < this.getPartCount());
      return this._partOpacities[partIndex];
    }
    /**
     * パーツの不透明度の取得(id)
     * @param partId パーツのＩｄ
     * @return パーツの不透明度
     */
    getPartOpacityById(partId) {
      const index = this.getPartIndex(partId);
      if (index < 0) {
        return 0;
      }
      return this.getPartOpacityByIndex(index);
    }
    /**
     * パラメータのインデックスの取得
     * @param パラメータID
     * @return パラメータのインデックス
     */
    getParameterIndex(parameterId) {
      let parameterIndex;
      const idCount = this._model.parameters.count;
      for (parameterIndex = 0; parameterIndex < idCount; ++parameterIndex) {
        if (parameterId != this._parameterIds[parameterIndex]) {
          continue;
        }
        return parameterIndex;
      }
      if (this._notExistParameterId.has(parameterId)) {
        return this._notExistParameterId.get(parameterId);
      }
      parameterIndex = this._model.parameters.count + this._notExistParameterId.size;
      this._notExistParameterId.set(parameterId, parameterIndex);
      this._notExistParameterValues.set(parameterIndex, null);
      return parameterIndex;
    }
    /**
     * パラメータの個数の取得
     * @return パラメータの個数
     */
    getParameterCount() {
      return this._model.parameters.count;
    }
    /**
     * パラメータの種類の取得
     * @param parameterIndex パラメータのインデックス
     * @return csmParameterType_Normal -> 通常のパラメータ
     *          csmParameterType_BlendShape -> ブレンドシェイプパラメータ
     */
    getParameterType(parameterIndex) {
      return this._model.parameters.types[parameterIndex];
    }
    /**
     * パラメータの最大値の取得
     * @param parameterIndex パラメータのインデックス
     * @return パラメータの最大値
     */
    getParameterMaximumValue(parameterIndex) {
      return this._model.parameters.maximumValues[parameterIndex];
    }
    /**
     * パラメータの最小値の取得
     * @param parameterIndex パラメータのインデックス
     * @return パラメータの最小値
     */
    getParameterMinimumValue(parameterIndex) {
      return this._model.parameters.minimumValues[parameterIndex];
    }
    /**
     * パラメータのデフォルト値の取得
     * @param parameterIndex パラメータのインデックス
     * @return パラメータのデフォルト値
     */
    getParameterDefaultValue(parameterIndex) {
      return this._model.parameters.defaultValues[parameterIndex];
    }
    /**
     * 指定したパラメータindexのIDを取得
     *
     * @param parameterIndex パラメータのインデックス
     * @return パラメータID
     */
    getParameterId(parameterIndex) {
      return CubismFramework.getIdManager().getId(
        this._model.parameters.ids[parameterIndex]
      );
    }
    /**
     * パラメータの値の取得
     * @param parameterIndex    パラメータのインデックス
     * @return パラメータの値
     */
    getParameterValueByIndex(parameterIndex) {
      if (this._notExistParameterValues.has(parameterIndex)) {
        return this._notExistParameterValues.get(parameterIndex);
      }
      CSM_ASSERT(
        0 <= parameterIndex && parameterIndex < this.getParameterCount()
      );
      return this._parameterValues[parameterIndex];
    }
    /**
     * パラメータの値の取得
     * @param parameterId    パラメータのID
     * @return パラメータの値
     */
    getParameterValueById(parameterId) {
      const parameterIndex = this.getParameterIndex(parameterId);
      return this.getParameterValueByIndex(parameterIndex);
    }
    /**
     * パラメータの値の設定
     * @param parameterIndex パラメータのインデックス
     * @param value パラメータの値
     * @param weight 重み
     */
    setParameterValueByIndex(parameterIndex, value, weight = 1) {
      if (this._notExistParameterValues.has(parameterIndex)) {
        this._notExistParameterValues.set(
          parameterIndex,
          weight == 1 ? value : this._notExistParameterValues.get(parameterIndex) * (1 - weight) + value * weight
        );
        return;
      }
      CSM_ASSERT(
        0 <= parameterIndex && parameterIndex < this.getParameterCount()
      );
      if (this.isRepeat(parameterIndex)) {
        value = this.getParameterRepeatValue(parameterIndex, value);
      } else {
        value = this.getParameterClampValue(parameterIndex, value);
      }
      this._parameterValues[parameterIndex] = weight == 1 ? value : this._parameterValues[parameterIndex] = this._parameterValues[parameterIndex] * (1 - weight) + value * weight;
    }
    /**
     * パラメータの値の設定
     * @param parameterId パラメータのID
     * @param value パラメータの値
     * @param weight 重み
     */
    setParameterValueById(parameterId, value, weight = 1) {
      const index = this.getParameterIndex(parameterId);
      this.setParameterValueByIndex(index, value, weight);
    }
    /**
     * パラメータの値の加算(index)
     * @param parameterIndex パラメータインデックス
     * @param value 加算する値
     * @param weight 重み
     */
    addParameterValueByIndex(parameterIndex, value, weight = 1) {
      this.setParameterValueByIndex(
        parameterIndex,
        this.getParameterValueByIndex(parameterIndex) + value * weight
      );
    }
    /**
     * パラメータの値の加算(id)
     * @param parameterId パラメータＩＤ
     * @param value 加算する値
     * @param weight 重み
     */
    addParameterValueById(parameterId, value, weight = 1) {
      const index = this.getParameterIndex(parameterId);
      this.addParameterValueByIndex(index, value, weight);
    }
    /**
     * Gets whether the parameter has the repeat setting.
     *
     * @param parameterIndex Parameter index
     *
     * @return true if it is set, otherwise returns false.
     */
    isRepeat(parameterIndex) {
      if (this._notExistParameterValues.has(parameterIndex)) {
        return false;
      }
      CSM_ASSERT(
        0 <= parameterIndex && parameterIndex < this.getParameterCount()
      );
      let isRepeat;
      if (this._isOverriddenParameterRepeat || this._userParameterRepeatDataList[parameterIndex].isOverridden) {
        isRepeat = this._userParameterRepeatDataList[parameterIndex].isParameterRepeated;
      } else {
        isRepeat = this._model.parameters.repeats[parameterIndex] != 0;
      }
      return isRepeat;
    }
    /**
     * Returns the calculated result ensuring the value falls within the parameter's range.
     *
     * @param parameterIndex Parameter index
     * @param value Parameter value
     *
     * @return a value that falls within the parameter’s range. If the parameter does not exist, returns it as is.
     */
    getParameterRepeatValue(parameterIndex, value) {
      if (this._notExistParameterValues.has(parameterIndex)) {
        return value;
      }
      CSM_ASSERT(
        0 <= parameterIndex && parameterIndex < this.getParameterCount()
      );
      const maxValue = this._model.parameters.maximumValues[parameterIndex];
      const minValue = this._model.parameters.minimumValues[parameterIndex];
      const valueSize = maxValue - minValue;
      if (maxValue < value) {
        const overValue = CubismMath.mod(value - maxValue, valueSize);
        if (!Number.isNaN(overValue)) {
          value = minValue + overValue;
        } else {
          value = maxValue;
        }
      }
      if (value < minValue) {
        const overValue = CubismMath.mod(minValue - value, valueSize);
        if (!Number.isNaN(overValue)) {
          value = maxValue - overValue;
        } else {
          value = minValue;
        }
      }
      return value;
    }
    /**
     * Returns the result of clamping the value to ensure it falls within the parameter's range.
     *
     * @param parameterIndex Parameter index
     * @param value Parameter value
     *
     * @return the clamped value. If the parameter does not exist, returns it as is.
     */
    getParameterClampValue(parameterIndex, value) {
      if (this._notExistParameterValues.has(parameterIndex)) {
        return value;
      }
      CSM_ASSERT(
        0 <= parameterIndex && parameterIndex < this.getParameterCount()
      );
      const maxValue = this._model.parameters.maximumValues[parameterIndex];
      const minValue = this._model.parameters.minimumValues[parameterIndex];
      return CubismMath.clamp(value, minValue, maxValue);
    }
    /**
     * Returns the repeat of the parameter.
     *
     * @param parameterIndex Parameter index
     *
     * @return the raw data parameter repeat from the Cubism Core.
     */
    getParameterRepeats(parameterIndex) {
      return this._model.parameters.repeats[parameterIndex] != 0;
    }
    /**
     * パラメータの値の乗算
     * @param parameterId パラメータのID
     * @param value 乗算する値
     * @param weight 重み
     */
    multiplyParameterValueById(parameterId, value, weight = 1) {
      const index = this.getParameterIndex(parameterId);
      this.multiplyParameterValueByIndex(index, value, weight);
    }
    /**
     * パラメータの値の乗算
     * @param parameterIndex パラメータのインデックス
     * @param value 乗算する値
     * @param weight 重み
     */
    multiplyParameterValueByIndex(parameterIndex, value, weight = 1) {
      this.setParameterValueByIndex(
        parameterIndex,
        this.getParameterValueByIndex(parameterIndex) * (1 + (value - 1) * weight)
      );
    }
    /**
     * Drawableのインデックスの取得
     * @param drawableId DrawableのID
     * @return Drawableのインデックス
     */
    getDrawableIndex(drawableId) {
      const drawableCount = this._model.drawables.count;
      for (let drawableIndex = 0; drawableIndex < drawableCount; ++drawableIndex) {
        if (this._drawableIds[drawableIndex] == drawableId) {
          return drawableIndex;
        }
      }
      return -1;
    }
    /**
     * Drawableの個数の取得
     * @return drawableの個数
     */
    getDrawableCount() {
      const drawableCount = this._model.drawables.count;
      return drawableCount;
    }
    /**
     * DrawableのIDを取得する
     * @param drawableIndex Drawableのインデックス
     * @return drawableのID
     */
    getDrawableId(drawableIndex) {
      const parameterIds = this._model.drawables.ids;
      return CubismFramework.getIdManager().getId(parameterIds[drawableIndex]);
    }
    /**
     * Drawableの描画順リストの取得
     * @return Drawableの描画順リスト
     */
    getRenderOrders() {
      const renderOrders = this._model.getRenderOrders();
      return renderOrders;
    }
    /**
     * Drawableのテクスチャインデックスの取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableのテクスチャインデックス
     */
    getDrawableTextureIndex(drawableIndex) {
      const textureIndices = this._model.drawables.textureIndices;
      return textureIndices[drawableIndex];
    }
    /**
     * DrawableのVertexPositionsの変化情報の取得
     *
     * 直近のCubismModel.update関数でDrawableの頂点情報が変化したかを取得する。
     *
     * @param   drawableIndex   Drawableのインデックス
     * @return  true    Drawableの頂点情報が直近のCubismModel.update関数で変化した
     *          false   Drawableの頂点情報が直近のCubismModel.update関数で変化していない
     */
    getDrawableDynamicFlagVertexPositionsDidChange(drawableIndex) {
      const dynamicFlags = this._model.drawables.dynamicFlags;
      return Live2DCubismCore.Utils.hasVertexPositionsDidChangeBit(
        dynamicFlags[drawableIndex]
      );
    }
    /**
     * Drawableの頂点インデックスの個数の取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの頂点インデックスの個数
     */
    getDrawableVertexIndexCount(drawableIndex) {
      const indexCounts = this._model.drawables.indexCounts;
      return indexCounts[drawableIndex];
    }
    /**
     * Drawableの頂点の個数の取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの頂点の個数
     */
    getDrawableVertexCount(drawableIndex) {
      const vertexCounts = this._model.drawables.vertexCounts;
      return vertexCounts[drawableIndex];
    }
    /**
     * Drawableの頂点リストの取得
     * @param drawableIndex drawableのインデックス
     * @return drawableの頂点リスト
     */
    getDrawableVertices(drawableIndex) {
      return this.getDrawableVertexPositions(drawableIndex);
    }
    /**
     * Drawableの頂点インデックスリストの取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの頂点インデックスリスト
     */
    getDrawableVertexIndices(drawableIndex) {
      const indicesArray = this._model.drawables.indices;
      return indicesArray[drawableIndex];
    }
    /**
     * Drawableの頂点リストの取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの頂点リスト
     */
    getDrawableVertexPositions(drawableIndex) {
      const verticesArray = this._model.drawables.vertexPositions;
      return verticesArray[drawableIndex];
    }
    /**
     * Drawableの頂点のUVリストの取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの頂点UVリスト
     */
    getDrawableVertexUvs(drawableIndex) {
      const uvsArray = this._model.drawables.vertexUvs;
      return uvsArray[drawableIndex];
    }
    /**
     * Drawableの不透明度の取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの不透明度
     */
    getDrawableOpacity(drawableIndex) {
      const opacities = this._model.drawables.opacities;
      return opacities[drawableIndex];
    }
    /**
     * Drawableの乗算色の取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの乗算色(RGBA)
     * スクリーン色はRGBAで取得されるが、Aは必ず0
     */
    getDrawableMultiplyColor(drawableIndex) {
      if (this._drawableMultiplyColors == null) {
        this._drawableMultiplyColors = new Array(
          this._model.drawables.count
        );
        this._drawableMultiplyColors.fill(new CubismTextureColor());
      }
      const multiplyColors = this._model.drawables.multiplyColors;
      const index = drawableIndex * 4;
      this._drawableMultiplyColors[drawableIndex].r = multiplyColors[index];
      this._drawableMultiplyColors[drawableIndex].g = multiplyColors[index + 1];
      this._drawableMultiplyColors[drawableIndex].b = multiplyColors[index + 2];
      this._drawableMultiplyColors[drawableIndex].a = multiplyColors[index + 3];
      return this._drawableMultiplyColors[drawableIndex];
    }
    /**
     * Drawableのスクリーン色の取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableのスクリーン色(RGBA)
     * スクリーン色はRGBAで取得されるが、Aは必ず0
     */
    getDrawableScreenColor(drawableIndex) {
      if (this._drawableScreenColors == null) {
        this._drawableScreenColors = new Array(
          this._model.drawables.count
        );
        this._drawableScreenColors.fill(new CubismTextureColor());
      }
      const screenColors = this._model.drawables.screenColors;
      const index = drawableIndex * 4;
      this._drawableScreenColors[drawableIndex].r = screenColors[index];
      this._drawableScreenColors[drawableIndex].g = screenColors[index + 1];
      this._drawableScreenColors[drawableIndex].b = screenColors[index + 2];
      this._drawableScreenColors[drawableIndex].a = screenColors[index + 3];
      return this._drawableScreenColors[drawableIndex];
    }
    /**
     * Offscreenの乗算色の取得
     * @param offscreenIndex Offscreenのインデックス
     * @return Offscreenの乗算色(RGBA)
     * スクリーン色はRGBAで取得されるが、Aは必ず0
     */
    getOffscreenMultiplyColor(offscreenIndex) {
      if (this._offscreenMultiplyColors == null) {
        this._offscreenMultiplyColors = new Array(
          this._model.offscreens.count
        );
        this._offscreenMultiplyColors.fill(new CubismTextureColor());
      }
      const multiplyColors = this._model.offscreens.multiplyColors;
      const index = offscreenIndex * 4;
      this._offscreenMultiplyColors[offscreenIndex].r = multiplyColors[index];
      this._offscreenMultiplyColors[offscreenIndex].g = multiplyColors[index + 1];
      this._offscreenMultiplyColors[offscreenIndex].b = multiplyColors[index + 2];
      this._offscreenMultiplyColors[offscreenIndex].a = multiplyColors[index + 3];
      return this._offscreenMultiplyColors[offscreenIndex];
    }
    /**
     * Offscreenのスクリーン色の取得
     * @param offscreenIndex Offscreenのインデックス
     * @return Offscreenのスクリーン色(RGBA)
     * スクリーン色はRGBAで取得されるが、Aは必ず0
     */
    getOffscreenScreenColor(offscreenIndex) {
      if (this._offscreenScreenColors == null) {
        this._offscreenScreenColors = new Array(
          this._model.offscreens.count
        );
        this._offscreenScreenColors.fill(new CubismTextureColor());
      }
      const screenColors = this._model.offscreens.screenColors;
      const index = offscreenIndex * 4;
      this._offscreenScreenColors[offscreenIndex].r = screenColors[index];
      this._offscreenScreenColors[offscreenIndex].g = screenColors[index + 1];
      this._offscreenScreenColors[offscreenIndex].b = screenColors[index + 2];
      this._offscreenScreenColors[offscreenIndex].a = screenColors[index + 3];
      return this._offscreenScreenColors[offscreenIndex];
    }
    /**
     * Drawableの親パーツのインデックスの取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableの親パーツのインデックス
     */
    getDrawableParentPartIndex(drawableIndex) {
      return this._model.drawables.parentPartIndices[drawableIndex];
    }
    /**
     * Drawableのブレンドモードを取得
     * @param drawableIndex Drawableのインデックス
     * @return drawableのブレンドモード
     */
    getDrawableBlendMode(drawableIndex) {
      const constantFlags = this._model.drawables.constantFlags;
      return Live2DCubismCore.Utils.hasBlendAdditiveBit(
        constantFlags[drawableIndex]
      ) ? 1 /* CubismBlendMode_Additive */ : Live2DCubismCore.Utils.hasBlendMultiplicativeBit(
        constantFlags[drawableIndex]
      ) ? 2 /* CubismBlendMode_Multiplicative */ : 0 /* CubismBlendMode_Normal */;
    }
    /**
     * Drawableのカラーブレンドの取得(Cubism 5.3 以降)
     *
     * @param drawableIndex Drawableのインデックス
     * @return Drawableのカラーブレンド
     */
    getDrawableColorBlend(drawableIndex) {
      if (this._drawableColorBlends[drawableIndex] == -1 /* ColorBlend_None */) {
        this._drawableColorBlends[drawableIndex] = this._model.drawables.blendModes[drawableIndex] & 255;
      }
      return this._drawableColorBlends[drawableIndex];
    }
    /**
     * Drawableのアルファブレンドの取得(Cubism 5.3 以降)
     *
     * @param drawableIndex Drawableのインデックス
     * @return Drawableのアルファブレンド
     */
    getDrawableAlphaBlend(drawableIndex) {
      if (this._drawableAlphaBlends[drawableIndex] == -1 /* AlphaBlend_None */) {
        this._drawableAlphaBlends[drawableIndex] = this._model.drawables.blendModes[drawableIndex] >> 8 & 255;
      }
      return this._drawableAlphaBlends[drawableIndex];
    }
    /**
     * Drawableのマスクの反転使用の取得
     *
     * Drawableのマスク使用時の反転設定を取得する。
     * マスクを使用しない場合は無視される。
     *
     * @param drawableIndex Drawableのインデックス
     * @return Drawableの反転設定
     */
    getDrawableInvertedMaskBit(drawableIndex) {
      const constantFlags = this._model.drawables.constantFlags;
      return Live2DCubismCore.Utils.hasIsInvertedMaskBit(
        constantFlags[drawableIndex]
      );
    }
    /**
     * Drawableのクリッピングマスクリストの取得
     * @return Drawableのクリッピングマスクリスト
     */
    getDrawableMasks() {
      const masks = this._model.drawables.masks;
      return masks;
    }
    /**
     * Drawableのクリッピングマスクの個数リストの取得
     * @return Drawableのクリッピングマスクの個数リスト
     */
    getDrawableMaskCounts() {
      const maskCounts = this._model.drawables.maskCounts;
      return maskCounts;
    }
    /**
     * クリッピングマスクの使用状態
     *
     * @return true クリッピングマスクを使用している
     * @return false クリッピングマスクを使用していない
     */
    isUsingMasking() {
      for (let d = 0; d < this._model.drawables.count; ++d) {
        if (this._model.drawables.maskCounts[d] <= 0) {
          continue;
        }
        return true;
      }
      return false;
    }
    /**
     * Offscreenでクリッピングマスクを使用しているかどうかを取得
     *
     * @return true クリッピングマスクをオフスクリーンで使用している
     */
    isUsingMaskingForOffscreen() {
      for (let d = 0; d < this.getOffscreenCount(); ++d) {
        if (this._model.offscreens.maskCounts[d] <= 0) {
          continue;
        }
        return true;
      }
      return false;
    }
    /**
     * Drawableの表示情報を取得する
     *
     * @param drawableIndex Drawableのインデックス
     * @return true Drawableが表示
     * @return false Drawableが非表示
     */
    getDrawableDynamicFlagIsVisible(drawableIndex) {
      const dynamicFlags = this._model.drawables.dynamicFlags;
      return Live2DCubismCore.Utils.hasIsVisibleBit(dynamicFlags[drawableIndex]);
    }
    /**
     * DrawableのDrawOrderの変化情報の取得
     *
     * 直近のCubismModel.update関数でdrawableのdrawOrderが変化したかを取得する。
     * drawOrderはartMesh上で指定する0から1000の情報
     * @param drawableIndex drawableのインデックス
     * @return true drawableの不透明度が直近のCubismModel.update関数で変化した
     * @return false drawableの不透明度が直近のCubismModel.update関数で変化している
     */
    getDrawableDynamicFlagVisibilityDidChange(drawableIndex) {
      const dynamicFlags = this._model.drawables.dynamicFlags;
      return Live2DCubismCore.Utils.hasVisibilityDidChangeBit(
        dynamicFlags[drawableIndex]
      );
    }
    /**
     * Drawableの不透明度の変化情報の取得
     *
     * 直近のCubismModel.update関数でdrawableの不透明度が変化したかを取得する。
     *
     * @param drawableIndex drawableのインデックス
     * @return true Drawableの不透明度が直近のCubismModel.update関数で変化した
     * @return false Drawableの不透明度が直近のCubismModel.update関数で変化してない
     */
    getDrawableDynamicFlagOpacityDidChange(drawableIndex) {
      const dynamicFlags = this._model.drawables.dynamicFlags;
      return Live2DCubismCore.Utils.hasOpacityDidChangeBit(
        dynamicFlags[drawableIndex]
      );
    }
    /**
     * Drawableの描画順序の変化情報の取得
     *
     * 直近のCubismModel.update関数でDrawableの描画の順序が変化したかを取得する。
     *
     * @param drawableIndex Drawableのインデックス
     * @return true Drawableの描画の順序が直近のCubismModel.update関数で変化した
     * @return false Drawableの描画の順序が直近のCubismModel.update関数で変化してない
     */
    getDrawableDynamicFlagRenderOrderDidChange(drawableIndex) {
      const dynamicFlags = this._model.drawables.dynamicFlags;
      return Live2DCubismCore.Utils.hasRenderOrderDidChangeBit(
        dynamicFlags[drawableIndex]
      );
    }
    /**
     * Drawableの乗算色・スクリーン色の変化情報の取得
     *
     * 直近のCubismModel.update関数でDrawableの乗算色・スクリーン色が変化したかを取得する。
     *
     * @param drawableIndex Drawableのインデックス
     * @return true Drawableの乗算色・スクリーン色が直近のCubismModel.update関数で変化した
     * @return false Drawableの乗算色・スクリーン色が直近のCubismModel.update関数で変化してない
     */
    getDrawableDynamicFlagBlendColorDidChange(drawableIndex) {
      const dynamicFlags = this._model.drawables.dynamicFlags;
      return Live2DCubismCore.Utils.hasBlendColorDidChangeBit(
        dynamicFlags[drawableIndex]
      );
    }
    /**
     * オフスクリーンの個数を取得する
     * @return オフスクリーンの個数
     */
    getOffscreenCount() {
      return this._model.offscreens.count;
    }
    /**
     * Offscreenのカラーブレンドの取得(Cubism 5.3 以降)
     *
     * @param offscreenIndex Offscreenのインデックス
     * @return Offscreenのカラーブレンド
     */
    getOffscreenColorBlend(offscreenIndex) {
      if (this._offscreenColorBlends[offscreenIndex] == -1 /* ColorBlend_None */) {
        this._offscreenColorBlends[offscreenIndex] = this._model.offscreens.blendModes[offscreenIndex] & 255;
      }
      return this._offscreenColorBlends[offscreenIndex];
    }
    /**
     * Offscreenのアルファブレンドの取得(Cubism 5.3 以降)
     *
     * @param offscreenIndex Offscreenのインデックス
     * @return Offscreenのアルファブレンド
     */
    getOffscreenAlphaBlend(offscreenIndex) {
      if (this._offscreenAlphaBlends[offscreenIndex] == -1 /* AlphaBlend_None */) {
        this._offscreenAlphaBlends[offscreenIndex] = this._model.offscreens.blendModes[offscreenIndex] >> 8 & 255;
      }
      return this._offscreenAlphaBlends[offscreenIndex];
    }
    /**
     * オフスクリーンのオーナーインデックス配列を取得する
     * @return オフスクリーンのオーナーインデックス配列
     */
    getOffscreenOwnerIndices() {
      return this._model.offscreens.ownerIndices;
    }
    /**
     * オフスクリーンの不透明度を取得
     * @param offscreenIndex オフスクリーンのインデックス
     * @return 不透明度
     */
    getOffscreenOpacity(offscreenIndex) {
      if (offscreenIndex < 0 || offscreenIndex >= this._model.offscreens.count) {
        return 1;
      }
      return this._model.offscreens.opacities[offscreenIndex];
    }
    /**
     * オフスクリーンのクリッピングマスクリストの取得
     * @return オフスクリーンのクリッピングマスクリスト
     */
    getOffscreenMasks() {
      return this._model.offscreens.masks;
    }
    /**
     * オフスクリーンのクリッピングマスクの個数リストの取得
     * @return オフスクリーンのクリッピングマスクの個数リスト
     */
    getOffscreenMaskCounts() {
      return this._model.offscreens.maskCounts;
    }
    /**
     * オフスクリーンのマスク反転設定を取得する
     * @param offscreenIndex オフスクリーンのインデックス
     * @return オフスクリーンのマスク反転設定
     */
    getOffscreenInvertedMask(offscreenIndex) {
      const constantFlags = this._model.offscreens.constantFlags;
      return Live2DCubismCore.Utils.hasIsInvertedMaskBit(
        constantFlags[offscreenIndex]
      );
    }
    /**
     * ブレンドモード使用判定
     * @return ブレンドモードを使用しているか
     */
    isBlendModeEnabled() {
      return this._isBlendModeEnabled;
    }
    /**
     * 保存されたパラメータの読み込み
     */
    loadParameters() {
      let parameterCount = this._model.parameters.count;
      const savedParameterCount = this._savedParameters.length;
      if (parameterCount > savedParameterCount) {
        parameterCount = savedParameterCount;
      }
      for (let i = 0; i < parameterCount; ++i) {
        this._parameterValues[i] = this._savedParameters[i];
      }
    }
    /**
     * 初期化する
     */
    initialize() {
      CSM_ASSERT(this._model);
      this._parameterValues = this._model.parameters.values;
      this._partOpacities = this._model.parts.opacities;
      this._offscreenOpacities = this._model.offscreens.opacities;
      this._parameterMaximumValues = this._model.parameters.maximumValues;
      this._parameterMinimumValues = this._model.parameters.minimumValues;
      {
        const parameterIds = this._model.parameters.ids;
        const parameterCount = this._model.parameters.count;
        this._parameterIds.length = parameterCount;
        this._userParameterRepeatDataList.length = parameterCount;
        for (let i = 0; i < parameterCount; ++i) {
          this._parameterIds[i] = CubismFramework.getIdManager().getId(
            parameterIds[i]
          );
          this._userParameterRepeatDataList[i] = new ParameterRepeatData(
            false,
            false
          );
        }
      }
      const partCount = this._model.parts.count;
      {
        const partIds = this._model.parts.ids;
        this._partIds.length = partCount;
        for (let i = 0; i < partCount; ++i) {
          this._partIds[i] = CubismFramework.getIdManager().getId(partIds[i]);
        }
      }
      {
        const drawableIds = this._model.drawables.ids;
        const drawableCount = this._model.drawables.count;
        this._userDrawableCullings.length = drawableCount;
        const userCulling = new CullingData(false, false);
        this._userOffscreenCullings.length = this._model.offscreens.count;
        const userOffscreenCulling = new CullingData(false, false);
        {
          for (let i = 0; i < drawableCount; ++i) {
            this._drawableIds.push(
              CubismFramework.getIdManager().getId(drawableIds[i])
            );
            this._userDrawableCullings[i] = userCulling;
          }
        }
        {
          for (let i = 0; i < this._model.offscreens.count; ++i) {
            this._userOffscreenCullings[i] = userOffscreenCulling;
          }
        }
        if (this.getOffscreenCount() > 0) {
          this._isBlendModeEnabled = true;
        } else {
          const blendModes = this._model.drawables.blendModes;
          for (let i = 0; i < drawableCount; ++i) {
            const colorBlendType = this.getDrawableColorBlend(i);
            const alphaBlendType = this.getDrawableAlphaBlend(i);
            if (!(colorBlendType == CubismColorBlend.ColorBlend_Normal && alphaBlendType == 0 /* AlphaBlend_Over */) && colorBlendType != CubismColorBlend.ColorBlend_AddCompatible && colorBlendType != CubismColorBlend.ColorBlend_MultiplyCompatible) {
              this._isBlendModeEnabled = true;
              break;
            }
          }
        }
        this.setupPartsHierarchy();
        const offscreenCount = this.getOffscreenCount();
        this._overrideMultiplyAndScreenColor.initialize(
          partCount,
          drawableCount,
          offscreenCount
        );
      }
    }
    /**
     * パーツ階層構造を取得する
     * @return パーツ階層構造の配列
     */
    getPartsHierarchy() {
      return this._partsHierarchy;
    }
    /**
     * パーツ階層構造をセットアップする
     */
    setupPartsHierarchy() {
      this._partsHierarchy.length = 0;
      const partCount = this.getPartCount();
      this._partsHierarchy.length = partCount;
      for (let i = 0; i < partCount; ++i) {
        const partInfo = new CubismModelPartInfo();
        this._partsHierarchy[i] = partInfo;
      }
      for (let i = 0; i < partCount; ++i) {
        const parentPartIndex = this.getPartParentPartIndices()[i];
        if (parentPartIndex === NoParentIndex) {
          continue;
        }
        for (let partIndex = 0; partIndex < this._partsHierarchy.length; ++partIndex) {
          if (partIndex === parentPartIndex) {
            const objectInfo = new CubismModelObjectInfo(
              i,
              1 /* CubismModelObjectType_Parts */
            );
            this._partsHierarchy[partIndex].objects.push(objectInfo);
            break;
          }
        }
      }
      const drawableCount = this.getDrawableCount();
      for (let i = 0; i < drawableCount; ++i) {
        const parentPartIndex = this.getDrawableParentPartIndex(i);
        if (parentPartIndex === NoParentIndex) {
          continue;
        }
        for (let partIndex = 0; partIndex < this._partsHierarchy.length; ++partIndex) {
          if (partIndex === parentPartIndex) {
            const objectInfo = new CubismModelObjectInfo(
              i,
              0 /* CubismModelObjectType_Drawable */
            );
            this._partsHierarchy[partIndex].objects.push(objectInfo);
            break;
          }
        }
      }
      for (let i = 0; i < this._partsHierarchy.length; ++i) {
        this.getPartChildDrawObjects(i);
      }
    }
    /**
     * 指定したパーツの子描画オブジェクト情報を取得・構築する
     * @param partInfoIndex パーツ情報のインデックス
     * @return PartChildDrawObjects
     */
    getPartChildDrawObjects(partInfoIndex) {
      if (this._partsHierarchy[partInfoIndex].getChildObjectCount() < 1) {
        return this._partsHierarchy[partInfoIndex].childDrawObjects;
      }
      const childDrawObjects = this._partsHierarchy[partInfoIndex].childDrawObjects;
      if (childDrawObjects.drawableIndices.length !== 0 || childDrawObjects.offscreenIndices.length !== 0) {
        return childDrawObjects;
      }
      const objects = this._partsHierarchy[partInfoIndex].objects;
      for (let i = 0; i < objects.length; ++i) {
        const obj = objects[i];
        if (obj.objectType === 1 /* CubismModelObjectType_Parts */) {
          this.getPartChildDrawObjects(obj.objectIndex);
          const childToChildDrawObjects = this._partsHierarchy[obj.objectIndex].childDrawObjects;
          childDrawObjects.drawableIndices.push(
            ...childToChildDrawObjects.drawableIndices
          );
          childDrawObjects.offscreenIndices.push(
            ...childToChildDrawObjects.offscreenIndices
          );
          const offscreenIndices = this.getOffscreenIndices();
          const offscreenIndex = offscreenIndices ? offscreenIndices[obj.objectIndex] : NoOffscreenIndex;
          if (offscreenIndex !== NoOffscreenIndex) {
            childDrawObjects.offscreenIndices.push(offscreenIndex);
          }
        } else if (obj.objectType === 0 /* CubismModelObjectType_Drawable */) {
          childDrawObjects.drawableIndices.push(obj.objectIndex);
        }
      }
      return childDrawObjects;
    }
    /**
     * パーツのオフスクリーンインデックス配列を取得
     * @return Int32Array offscreenIndices
     */
    getOffscreenIndices() {
      return this._model.parts.offscreenIndices;
    }
    /**
     * コンストラクタ
     * @param model モデル
     */
    constructor(model) {
      this._model = model;
      this._parameterValues = null;
      this._parameterMaximumValues = null;
      this._parameterMinimumValues = null;
      this._partOpacities = null;
      this._offscreenOpacities = null;
      this._savedParameters = new Array();
      this._parameterIds = new Array();
      this._drawableIds = new Array();
      this._partIds = new Array();
      this._isOverriddenParameterRepeat = true;
      this._isOverriddenCullings = false;
      this._modelOpacity = 1;
      this._overrideMultiplyAndScreenColor = new CubismModelMultiplyAndScreenColor(this);
      this._isBlendModeEnabled = false;
      this._drawableColorBlends = null;
      this._drawableAlphaBlends = null;
      this._offscreenColorBlends = null;
      this._offscreenAlphaBlends = null;
      this._drawableMultiplyColors = null;
      this._drawableScreenColors = null;
      this._offscreenMultiplyColors = null;
      this._offscreenScreenColors = null;
      this._userParameterRepeatDataList = new Array();
      this._userDrawableCullings = new Array();
      this._userOffscreenCullings = new Array();
      this._partsHierarchy = new Array();
      this._notExistPartId = /* @__PURE__ */ new Map();
      this._notExistParameterId = /* @__PURE__ */ new Map();
      this._notExistParameterValues = /* @__PURE__ */ new Map();
      this._notExistPartOpacities = /* @__PURE__ */ new Map();
      this._drawableColorBlends = new Array(
        model.drawables.count
      ).fill(-1 /* ColorBlend_None */);
      this._drawableAlphaBlends = new Array(
        model.drawables.count
      ).fill(-1 /* AlphaBlend_None */);
      this._offscreenColorBlends = new Array(
        model.offscreens.count
      ).fill(-1 /* ColorBlend_None */);
      this._offscreenAlphaBlends = new Array(
        model.offscreens.count
      ).fill(-1 /* AlphaBlend_None */);
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this._model.release();
      this._model = null;
      this._drawableColorBlends = null;
      this._drawableAlphaBlends = null;
      this._offscreenColorBlends = null;
      this._offscreenAlphaBlends = null;
      this._drawableMultiplyColors = null;
      this._drawableScreenColors = null;
      this._offscreenMultiplyColors = null;
      this._offscreenScreenColors = null;
    }
    // Offscreenのスクリーン色の配列
  };
  var Live2DCubismFramework33;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismModel = CubismModel;
  })(Live2DCubismFramework33 || (Live2DCubismFramework33 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismclippingmanager.ts
  var ColorChannelCount = 4;
  var ClippingMaskMaxCountOnDefault = 36;
  var ClippingMaskMaxCountOnMultiRenderTexture = 32;
  var CubismClippingManager = class {
    /**
     * コンストラクタ
     */
    constructor(clippingContextFactory) {
      this._renderTextureCount = 0;
      this._clippingMaskBufferSize = 256;
      this._clippingContextListForMask = new Array();
      this._clippingContextListForDraw = new Array();
      this._clippingContextListForOffscreen = new Array();
      this._tmpBoundsOnModel = new csmRect();
      this._tmpMatrix = new CubismMatrix44();
      this._tmpMatrixForMask = new CubismMatrix44();
      this._tmpMatrixForDraw = new CubismMatrix44();
      this._clearedMaskBufferFlags = new Array();
      this._clippingContexttConstructor = clippingContextFactory;
      this._channelColors = [
        new CubismTextureColor(1, 0, 0, 0),
        new CubismTextureColor(0, 1, 0, 0),
        new CubismTextureColor(0, 0, 1, 0),
        new CubismTextureColor(0, 0, 0, 1)
      ];
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      for (let i = 0; i < this._clippingContextListForMask.length; i++) {
        if (this._clippingContextListForMask[i]) {
          this._clippingContextListForMask[i].release();
          this._clippingContextListForMask[i] = void 0;
        }
        this._clippingContextListForMask[i] = null;
      }
      this._clippingContextListForMask = null;
      for (let i = 0; i < this._clippingContextListForDraw.length; i++) {
        this._clippingContextListForDraw[i] = null;
      }
      this._clippingContextListForDraw = null;
      for (let i = 0; i < this._channelColors.length; i++) {
        this._channelColors[i] = null;
      }
      this._channelColors = null;
      if (this._clearedMaskBufferFlags != null) {
        this._clearedMaskBufferFlags.length = 0;
      }
      this._clearedMaskBufferFlags = null;
    }
    /**
     * マネージャの初期化処理
     * クリッピングマスクを使う描画オブジェクトの登録を行う
     * @param model モデルのインスタンス
     * @param renderTextureCount バッファの生成数
     */
    initializeForDrawable(model, renderTextureCount) {
      if (renderTextureCount % 1 != 0) {
        CubismLogWarning(
          "The number of render textures must be specified as an integer. The decimal point is rounded down and corrected to an integer."
        );
        renderTextureCount = ~~renderTextureCount;
      }
      if (renderTextureCount < 1) {
        CubismLogWarning(
          "The number of render textures must be an integer greater than or equal to 1. Set the number of render textures to 1."
        );
      }
      this._renderTextureCount = renderTextureCount < 1 ? 1 : renderTextureCount;
      this._clearedMaskBufferFlags = new Array(this._renderTextureCount);
      this._clippingContextListForDraw.length = model.getDrawableCount();
      for (let i = 0; i < model.getDrawableCount(); i++) {
        if (model.getDrawableMaskCounts()[i] <= 0) {
          this._clippingContextListForDraw[i] = null;
          continue;
        }
        let clippingContext = this.findSameClip(
          model.getDrawableMasks()[i],
          model.getDrawableMaskCounts()[i]
        );
        if (clippingContext == null) {
          clippingContext = new this._clippingContexttConstructor(
            this,
            model.getDrawableMasks()[i],
            model.getDrawableMaskCounts()[i]
          );
          this._clippingContextListForMask.push(clippingContext);
        }
        clippingContext.addClippedDrawable(i);
        this._clippingContextListForDraw[i] = clippingContext;
      }
    }
    /**
     * オフスクリーン用の初期化処理
     *
     * @param model モデルのインスタンス
     * @param maskBufferCount オフスクリーン用のマスクバッファの数
     */
    initializeForOffscreen(model, maskBufferCount) {
      this._renderTextureCount = maskBufferCount;
      this._clearedMaskBufferFlags.length = this._renderTextureCount;
      for (let i = 0; i < this._renderTextureCount; ++i) {
        this._clearedMaskBufferFlags[i] = false;
      }
      this._clippingContextListForOffscreen.length = model.getOffscreenCount();
      for (let i = 0; i < model.getOffscreenCount(); ++i) {
        if (model.getOffscreenMaskCounts()[i] <= 0) {
          this._clippingContextListForOffscreen.push(null);
          continue;
        }
        let cc = this.findSameClip(
          model.getOffscreenMasks()[i],
          model.getOffscreenMaskCounts()[i]
        );
        if (cc == null) {
          cc = new this._clippingContexttConstructor(
            this,
            model.getOffscreenMasks()[i],
            model.getOffscreenMaskCounts()[i]
          );
          this._clippingContextListForMask.push(cc);
        }
        cc.addClippedOffscreen(i);
        this._clippingContextListForOffscreen[i] = cc;
      }
    }
    /**
     * 既にマスクを作っているかを確認
     * 作っている様であれば該当するクリッピングマスクのインスタンスを返す
     * 作っていなければNULLを返す
     * @param drawableMasks 描画オブジェクトをマスクする描画オブジェクトのリスト
     * @param drawableMaskCounts 描画オブジェクトをマスクする描画オブジェクトの数
     * @return 該当するクリッピングマスクが存在すればインスタンスを返し、なければNULLを返す
     */
    findSameClip(drawableMasks, drawableMaskCounts) {
      for (let i = 0; i < this._clippingContextListForMask.length; i++) {
        const clippingContext = this._clippingContextListForMask[i];
        const count = clippingContext._clippingIdCount;
        if (count != drawableMaskCounts) {
          continue;
        }
        let sameCount = 0;
        for (let j = 0; j < count; j++) {
          const clipId = clippingContext._clippingIdList[j];
          for (let k = 0; k < count; k++) {
            if (drawableMasks[k] == clipId) {
              sameCount++;
              break;
            }
          }
        }
        if (sameCount == count) {
          return clippingContext;
        }
      }
      return null;
    }
    /**
     * 高精細マスク処理用の行列を計算する
     * @param model モデルのインスタンス
     * @param isRightHanded 処理が右手系であるか
     */
    setupMatrixForHighPrecision(model, isRightHanded) {
      let usingClipCount = 0;
      for (let clipIndex = 0; clipIndex < this._clippingContextListForMask.length; clipIndex++) {
        const cc = this._clippingContextListForMask[clipIndex];
        this.calcClippedDrawableTotalBounds(model, cc);
        if (cc._isUsing) {
          usingClipCount++;
        }
      }
      if (usingClipCount > 0) {
        this.setupLayoutBounds(0);
        if (this._clearedMaskBufferFlags.length != this._renderTextureCount) {
          this._clearedMaskBufferFlags.length = this._renderTextureCount;
          for (let i = 0; i < this._renderTextureCount; i++) {
            this._clearedMaskBufferFlags[i] = false;
          }
        } else {
          for (let i = 0; i < this._renderTextureCount; i++) {
            this._clearedMaskBufferFlags[i] = false;
          }
        }
        for (let clipIndex = 0; clipIndex < this._clippingContextListForMask.length; clipIndex++) {
          const clipContext = this._clippingContextListForMask[clipIndex];
          const allClippedDrawRect = clipContext._allClippedDrawRect;
          const layoutBoundsOnTex01 = clipContext._layoutBounds;
          const margin = 0.05;
          let scaleX = 0;
          let scaleY = 0;
          const ppu = model.getPixelsPerUnit();
          const maskPixelSize = clipContext.getClippingManager().getClippingMaskBufferSize();
          const physicalMaskWidth = layoutBoundsOnTex01.width * maskPixelSize;
          const physicalMaskHeight = layoutBoundsOnTex01.height * maskPixelSize;
          this._tmpBoundsOnModel.setRect(allClippedDrawRect);
          if (this._tmpBoundsOnModel.width * ppu > physicalMaskWidth) {
            this._tmpBoundsOnModel.expand(allClippedDrawRect.width * margin, 0);
            scaleX = layoutBoundsOnTex01.width / this._tmpBoundsOnModel.width;
          } else {
            scaleX = ppu / physicalMaskWidth;
          }
          if (this._tmpBoundsOnModel.height * ppu > physicalMaskHeight) {
            this._tmpBoundsOnModel.expand(
              0,
              allClippedDrawRect.height * margin
            );
            scaleY = layoutBoundsOnTex01.height / this._tmpBoundsOnModel.height;
          } else {
            scaleY = ppu / physicalMaskHeight;
          }
          this.createMatrixForMask(
            isRightHanded,
            layoutBoundsOnTex01,
            scaleX,
            scaleY
          );
          clipContext._matrixForMask.setMatrix(this._tmpMatrixForMask.getArray());
          clipContext._matrixForDraw.setMatrix(this._tmpMatrixForDraw.getArray());
        }
      }
    }
    /**
     * オフスクリーンの高精細マスク処理用の行列を計算する
     *
     * @param model モデルのインスタンス
     * @param isRightHanded 処理が右手系であるか
     * @param mvp モデルビュー投影行列
     */
    setupMatrixForOffscreenHighPrecision(model, isRightHanded, mvp) {
      let usingClipCount = 0;
      for (let clipIndex = 0; clipIndex < this._clippingContextListForMask.length; clipIndex++) {
        const cc = this._clippingContextListForMask[clipIndex];
        this.calcClippedOffscreenTotalBounds(model, cc);
        if (cc._isUsing) {
          usingClipCount++;
        }
      }
      if (usingClipCount <= 0) {
        return;
      }
      this.setupLayoutBounds(0);
      if (this._clearedMaskBufferFlags.length != this._renderTextureCount) {
        this._clearedMaskBufferFlags.length = this._renderTextureCount;
        for (let i = 0; i < this._renderTextureCount; ++i) {
          this._clearedMaskBufferFlags[i] = false;
        }
      } else {
        for (let i = 0; i < this._renderTextureCount; ++i) {
          this._clearedMaskBufferFlags[i] = false;
        }
      }
      for (let clipIndex = 0; clipIndex < this._clippingContextListForMask.length; clipIndex++) {
        const clipContext = this._clippingContextListForMask[clipIndex];
        const allClippedDrawRect = clipContext._allClippedDrawRect;
        const layoutBoundsOnTex01 = clipContext._layoutBounds;
        const margin = 0.05;
        let scaleX = 0;
        let scaleY = 0;
        const ppu = model.getPixelsPerUnit();
        const maskPixel = clipContext.getClippingManager().getClippingMaskBufferSize();
        const physicalMaskWidth = layoutBoundsOnTex01.width * maskPixel;
        const physicalMaskHeight = layoutBoundsOnTex01.height * maskPixel;
        this._tmpBoundsOnModel.setRect(allClippedDrawRect);
        if (this._tmpBoundsOnModel.width * ppu > physicalMaskWidth) {
          this._tmpBoundsOnModel.expand(allClippedDrawRect.width * margin, 0);
          scaleX = layoutBoundsOnTex01.width / this._tmpBoundsOnModel.width;
        } else {
          scaleX = ppu / physicalMaskWidth;
        }
        if (this._tmpBoundsOnModel.height * ppu > physicalMaskHeight) {
          this._tmpBoundsOnModel.expand(0, allClippedDrawRect.height * margin);
          scaleY = layoutBoundsOnTex01.height / this._tmpBoundsOnModel.height;
        } else {
          scaleY = ppu / physicalMaskHeight;
        }
        this.createMatrixForMask(
          isRightHanded,
          layoutBoundsOnTex01,
          scaleX,
          scaleY
        );
        clipContext._matrixForMask.setMatrix(this._tmpMatrixForMask.getArray());
        clipContext._matrixForDraw.setMatrix(this._tmpMatrixForDraw.getArray());
        const invertMvp = mvp.getInvert();
        clipContext._matrixForDraw.multiplyByMatrix(invertMvp);
      }
    }
    /**
     * マスクを使う描画オブジェクトの全体の矩形を計算する。
     *
     * @param model モデルのインスタンス
     * @param clippingContext クリッピングコンテキスト
     */
    calcClippedOffscreenTotalBounds(model, clippingContext) {
      let clippedDrawTotalMinX = Number.MAX_VALUE, clippedDrawTotalMinY = Number.MAX_VALUE;
      let clippedDrawTotalMaxX = -Number.MAX_VALUE, clippedDrawTotalMaxY = -Number.MAX_VALUE;
      const clippedOffscreenCount = clippingContext._clippedOffscreenIndexList.length;
      const clippedOffscreenChildDrawableIndexList = new Array();
      for (let clippedOffscreenIndex = 0; clippedOffscreenIndex < clippedOffscreenCount; clippedOffscreenIndex++) {
        const offscreenIndex = clippingContext._clippedOffscreenIndexList[clippedOffscreenIndex];
        this.getOffscreenChildDrawableIndexList(
          model,
          offscreenIndex,
          clippedOffscreenChildDrawableIndexList
        );
      }
      const childDrawableCount = clippedOffscreenChildDrawableIndexList.length;
      for (let childDrawableIndex = 0; childDrawableIndex < childDrawableCount; childDrawableIndex++) {
        const drawableVertexCount = model.getDrawableVertexCount(
          clippedOffscreenChildDrawableIndexList[childDrawableIndex]
        );
        const drawableVertexes = model.getDrawableVertices(
          clippedOffscreenChildDrawableIndexList[childDrawableIndex]
        );
        let minX = Number.MAX_VALUE, minY = Number.MAX_VALUE;
        let maxX = -Number.MAX_VALUE, maxY = -Number.MAX_VALUE;
        const loop = drawableVertexCount * Constant.vertexStep;
        for (let pi = Constant.vertexOffset; pi < loop; pi += Constant.vertexStep) {
          const x = drawableVertexes[pi];
          const y = drawableVertexes[pi + 1];
          if (x < minX) minX = x;
          if (x > maxX) maxX = x;
          if (y < minY) minY = y;
          if (y > maxY) maxY = y;
        }
        if (minX == Number.MAX_VALUE) continue;
        if (minX < clippedDrawTotalMinX) clippedDrawTotalMinX = minX;
        if (minY < clippedDrawTotalMinY) clippedDrawTotalMinY = minY;
        if (maxX > clippedDrawTotalMaxX) clippedDrawTotalMaxX = maxX;
        if (maxY > clippedDrawTotalMaxY) clippedDrawTotalMaxY = maxY;
      }
      if (clippedDrawTotalMinX == Number.MAX_VALUE) {
        clippingContext._allClippedDrawRect.x = 0;
        clippingContext._allClippedDrawRect.y = 0;
        clippingContext._allClippedDrawRect.width = 0;
        clippingContext._allClippedDrawRect.height = 0;
        clippingContext._isUsing = false;
      } else {
        clippingContext._isUsing = true;
        const w = clippedDrawTotalMaxX - clippedDrawTotalMinX;
        const h = clippedDrawTotalMaxY - clippedDrawTotalMinY;
        clippingContext._allClippedDrawRect.x = clippedDrawTotalMinX;
        clippingContext._allClippedDrawRect.y = clippedDrawTotalMinY;
        clippingContext._allClippedDrawRect.width = w;
        clippingContext._allClippedDrawRect.height = h;
      }
    }
    /**
     * マスクを使う描画オブジェクトの全体の矩形を計算する。
     *
     * @param model モデルのインスタンス
     * @param offscreenIndex オフスクリーンのインデックス
     * @param childDrawableIndexList オフスクリーンの子Drawableのインデックスリスト
     */
    getOffscreenChildDrawableIndexList(model, offscreenIndex, childDrawableIndexList) {
      const ownerIndex = model.getOffscreenOwnerIndices()[offscreenIndex];
      this.getPartChildDrawableIndexList(
        model,
        ownerIndex,
        childDrawableIndexList
      );
    }
    /**
     * パーツの子Drawableのインデックスリストを取得する。
     *
     * @param model モデルのインスタンス
     * @param partIndex パーツのインデックス
     * @param childDrawableIndexList パーツの子Drawableのインデックスリスト
     */
    getPartChildDrawableIndexList(model, partIndex, childDrawableIndexList) {
      const childDrawObjects = model.getPartsHierarchy()[partIndex].childDrawObjects;
      childDrawableIndexList.push(...childDrawObjects.drawableIndices);
      for (let i = 0; i < childDrawObjects.offscreenIndices.length; ++i) {
        this.getOffscreenChildDrawableIndexList(
          model,
          childDrawObjects.offscreenIndices[i],
          childDrawableIndexList
        );
      }
    }
    /**
     * マスク作成・描画用の行列を作成する。
     * @param isRightHanded 座標を右手系として扱うかを指定
     * @param layoutBoundsOnTex01 マスクを収める領域
     * @param scaleX 描画オブジェクトの伸縮率
     * @param scaleY 描画オブジェクトの伸縮率
     */
    createMatrixForMask(isRightHanded, layoutBoundsOnTex01, scaleX, scaleY) {
      this._tmpMatrix.loadIdentity();
      {
        this._tmpMatrix.translateRelative(-1, -1);
        this._tmpMatrix.scaleRelative(2, 2);
      }
      {
        this._tmpMatrix.translateRelative(
          layoutBoundsOnTex01.x,
          layoutBoundsOnTex01.y
        );
        this._tmpMatrix.scaleRelative(scaleX, scaleY);
        this._tmpMatrix.translateRelative(
          -this._tmpBoundsOnModel.x,
          -this._tmpBoundsOnModel.y
        );
      }
      this._tmpMatrixForMask.setMatrix(this._tmpMatrix.getArray());
      this._tmpMatrix.loadIdentity();
      {
        this._tmpMatrix.translateRelative(
          layoutBoundsOnTex01.x,
          layoutBoundsOnTex01.y * (isRightHanded ? -1 : 1)
        );
        this._tmpMatrix.scaleRelative(
          scaleX,
          scaleY * (isRightHanded ? -1 : 1)
        );
        this._tmpMatrix.translateRelative(
          -this._tmpBoundsOnModel.x,
          -this._tmpBoundsOnModel.y
        );
      }
      this._tmpMatrixForDraw.setMatrix(this._tmpMatrix.getArray());
    }
    /**
     * クリッピングコンテキストを配置するレイアウト
     * 指定された数のレンダーテクスチャを極力いっぱいに使ってマスクをレイアウトする
     * マスクグループの数が4以下ならRGBA各チャンネルに一つずつマスクを配置し、5以上6以下ならRGBAを2,2,1,1と配置する。
     *
     * @param usingClipCount 配置するクリッピングコンテキストの数
     */
    setupLayoutBounds(usingClipCount) {
      const useClippingMaskMaxCount = this._renderTextureCount <= 1 ? ClippingMaskMaxCountOnDefault : ClippingMaskMaxCountOnMultiRenderTexture * this._renderTextureCount;
      if (usingClipCount <= 0 || usingClipCount > useClippingMaskMaxCount) {
        if (usingClipCount > useClippingMaskMaxCount) {
          CubismLogError(
            "not supported mask count : {0}\n[Details] render texture count : {1}, mask count : {2}",
            usingClipCount - useClippingMaskMaxCount,
            this._renderTextureCount,
            usingClipCount
          );
        }
        for (let index = 0; index < this._clippingContextListForMask.length; index++) {
          const clipContext = this._clippingContextListForMask[index];
          clipContext._layoutChannelIndex = 0;
          clipContext._layoutBounds.x = 0;
          clipContext._layoutBounds.y = 0;
          clipContext._layoutBounds.width = 1;
          clipContext._layoutBounds.height = 1;
          clipContext._bufferIndex = 0;
        }
        return;
      }
      const layoutCountMaxValue = this._renderTextureCount <= 1 ? 9 : 8;
      let countPerSheetDiv = usingClipCount / this._renderTextureCount;
      const reduceLayoutTextureCount = usingClipCount % this._renderTextureCount;
      countPerSheetDiv = Math.ceil(countPerSheetDiv);
      let divCount = countPerSheetDiv / ColorChannelCount;
      const modCount = countPerSheetDiv % ColorChannelCount;
      divCount = ~~divCount;
      let curClipIndex = 0;
      for (let renderTextureIndex = 0; renderTextureIndex < this._renderTextureCount; renderTextureIndex++) {
        for (let channelIndex = 0; channelIndex < ColorChannelCount; channelIndex++) {
          let layoutCount = divCount + (channelIndex < modCount ? 1 : 0);
          const checkChannelIndex = modCount + (divCount < 1 ? -1 : 0);
          if (channelIndex == checkChannelIndex && reduceLayoutTextureCount > 0) {
            layoutCount -= !(renderTextureIndex < reduceLayoutTextureCount) ? 1 : 0;
          }
          if (layoutCount == 0) {
          } else if (layoutCount == 1) {
            const clipContext = this._clippingContextListForMask[curClipIndex++];
            clipContext._layoutChannelIndex = channelIndex;
            clipContext._layoutBounds.x = 0;
            clipContext._layoutBounds.y = 0;
            clipContext._layoutBounds.width = 1;
            clipContext._layoutBounds.height = 1;
            clipContext._bufferIndex = renderTextureIndex;
          } else if (layoutCount == 2) {
            for (let i = 0; i < layoutCount; i++) {
              let xpos = i % 2;
              xpos = ~~xpos;
              const cc = this._clippingContextListForMask[curClipIndex++];
              cc._layoutChannelIndex = channelIndex;
              cc._layoutBounds.x = xpos * 0.5;
              cc._layoutBounds.y = 0;
              cc._layoutBounds.width = 0.5;
              cc._layoutBounds.height = 1;
              cc._bufferIndex = renderTextureIndex;
            }
          } else if (layoutCount <= 4) {
            for (let i = 0; i < layoutCount; i++) {
              let xpos = i % 2;
              let ypos = i / 2;
              xpos = ~~xpos;
              ypos = ~~ypos;
              const cc = this._clippingContextListForMask[curClipIndex++];
              cc._layoutChannelIndex = channelIndex;
              cc._layoutBounds.x = xpos * 0.5;
              cc._layoutBounds.y = ypos * 0.5;
              cc._layoutBounds.width = 0.5;
              cc._layoutBounds.height = 0.5;
              cc._bufferIndex = renderTextureIndex;
            }
          } else if (layoutCount <= layoutCountMaxValue) {
            for (let i = 0; i < layoutCount; i++) {
              let xpos = i % 3;
              let ypos = i / 3;
              xpos = ~~xpos;
              ypos = ~~ypos;
              const cc = this._clippingContextListForMask[curClipIndex++];
              cc._layoutChannelIndex = channelIndex;
              cc._layoutBounds.x = xpos / 3;
              cc._layoutBounds.y = ypos / 3;
              cc._layoutBounds.width = 1 / 3;
              cc._layoutBounds.height = 1 / 3;
              cc._bufferIndex = renderTextureIndex;
            }
          } else {
            CubismLogError(
              "not supported mask count : {0}\n[Details] render texture count : {1}, mask count : {2}",
              usingClipCount - useClippingMaskMaxCount,
              this._renderTextureCount,
              usingClipCount
            );
            for (let index = 0; index < layoutCount; index++) {
              const cc = this._clippingContextListForMask[curClipIndex++];
              cc._layoutChannelIndex = 0;
              cc._layoutBounds.x = 0;
              cc._layoutBounds.y = 0;
              cc._layoutBounds.width = 1;
              cc._layoutBounds.height = 1;
              cc._bufferIndex = 0;
            }
          }
        }
      }
    }
    /**
     * マスクされる描画オブジェクト群全体を囲む矩形（モデル座標系）を計算する
     * @param model モデルのインスタンス
     * @param clippingContext クリッピングマスクのコンテキスト
     */
    calcClippedDrawableTotalBounds(model, clippingContext) {
      let clippedDrawTotalMinX = Number.MAX_VALUE;
      let clippedDrawTotalMinY = Number.MAX_VALUE;
      let clippedDrawTotalMaxX = Number.MIN_VALUE;
      let clippedDrawTotalMaxY = Number.MIN_VALUE;
      const clippedDrawCount = clippingContext._clippedDrawableIndexList.length;
      for (let clippedDrawableIndex = 0; clippedDrawableIndex < clippedDrawCount; clippedDrawableIndex++) {
        const drawableIndex = clippingContext._clippedDrawableIndexList[clippedDrawableIndex];
        const drawableVertexCount = model.getDrawableVertexCount(drawableIndex);
        const drawableVertexes = model.getDrawableVertices(drawableIndex);
        let minX = Number.MAX_VALUE;
        let minY = Number.MAX_VALUE;
        let maxX = -Number.MAX_VALUE;
        let maxY = -Number.MAX_VALUE;
        const loop = drawableVertexCount * Constant.vertexStep;
        for (let pi = Constant.vertexOffset; pi < loop; pi += Constant.vertexStep) {
          const x = drawableVertexes[pi];
          const y = drawableVertexes[pi + 1];
          if (x < minX) {
            minX = x;
          }
          if (x > maxX) {
            maxX = x;
          }
          if (y < minY) {
            minY = y;
          }
          if (y > maxY) {
            maxY = y;
          }
        }
        if (minX == Number.MAX_VALUE) {
          continue;
        }
        if (minX < clippedDrawTotalMinX) {
          clippedDrawTotalMinX = minX;
        }
        if (minY < clippedDrawTotalMinY) {
          clippedDrawTotalMinY = minY;
        }
        if (maxX > clippedDrawTotalMaxX) {
          clippedDrawTotalMaxX = maxX;
        }
        if (maxY > clippedDrawTotalMaxY) {
          clippedDrawTotalMaxY = maxY;
        }
        if (clippedDrawTotalMinX == Number.MAX_VALUE) {
          clippingContext._allClippedDrawRect.x = 0;
          clippingContext._allClippedDrawRect.y = 0;
          clippingContext._allClippedDrawRect.width = 0;
          clippingContext._allClippedDrawRect.height = 0;
          clippingContext._isUsing = false;
        } else {
          clippingContext._isUsing = true;
          const w = clippedDrawTotalMaxX - clippedDrawTotalMinX;
          const h = clippedDrawTotalMaxY - clippedDrawTotalMinY;
          clippingContext._allClippedDrawRect.x = clippedDrawTotalMinX;
          clippingContext._allClippedDrawRect.y = clippedDrawTotalMinY;
          clippingContext._allClippedDrawRect.width = w;
          clippingContext._allClippedDrawRect.height = h;
        }
      }
    }
    /**
     * 画面描画に使用するクリッピングマスクのリストを取得する
     * @return 画面描画に使用するクリッピングマスクのリスト
     */
    getClippingContextListForDraw() {
      return this._clippingContextListForDraw;
    }
    getClippingContextListForOffscreen() {
      return this._clippingContextListForOffscreen;
    }
    /**
     * クリッピングマスクバッファのサイズを取得する
     * @return クリッピングマスクバッファのサイズ
     */
    getClippingMaskBufferSize() {
      return this._clippingMaskBufferSize;
    }
    /**
     * このバッファのレンダーテクスチャの枚数を取得する
     * @return このバッファのレンダーテクスチャの枚数
     */
    getRenderTextureCount() {
      return this._renderTextureCount;
    }
    /**
     * カラーチャンネル（RGBA）のフラグを取得する
     * @param channelNo カラーチャンネル（RGBA）の番号（0:R, 1:G, 2:B, 3:A）
     */
    getChannelFlagAsColor(channelNo) {
      return this._channelColors[channelNo];
    }
    /**
     * クリッピングマスクバッファのサイズを設定する
     * @param size クリッピングマスクバッファのサイズ
     */
    setClippingMaskBufferSize(size) {
      this._clippingMaskBufferSize = size;
    }
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismshader_webgl.ts
  var VertShaderSrcPath = "vertshadersrc.vert";
  var VertShaderSrcMaskedPath = "vertshadersrcmasked.vert";
  var VertShaderSrcSetupMaskPath = "vertshadersrcsetupmask.vert";
  var FragShaderSrcSetupMaskPath = "fragshadersrcsetupmask.frag";
  var FragShaderSrcPremultipliedAlphaPath = "fragshadersrcpremultipliedalpha.frag";
  var FragShaderSrcMaskPremultipliedAlphaPath = "fragshadersrcmaskpremultipliedalpha.frag";
  var FragShaderSrcMaskInvertedPremultipliedAlphaPath = "fragshadersrcmaskinvertedpremultipliedalpha.frag";
  var VertShaderSrcCopyPath = "vertshadersrccopy.vert";
  var FragShaderSrcCopyPath = "fragshadersrccopy.frag";
  var FragShaderSrcColorBlendPath = "fragshadersrccolorblend.frag";
  var FragShaderSrcAlphaBlendPath = "fragshadersrcalphablend.frag";
  var VertShaderSrcBlendPath = "vertshadersrcblend.vert";
  var FragShaderSrcBlendPath = "fragshadersrcpremultipliedalphablend.frag";
  var ColorBlendPrefix = "ColorBlend_";
  var AlphaBlendPrefix = "AlphaBlend_";
  var s_instance;
  var s_renderTargetVertexArray = new Float32Array([
    -1,
    -1,
    1,
    -1,
    -1,
    1,
    1,
    1
  ]);
  var s_renderTargetUvArray = new Float32Array([
    0,
    0,
    1,
    0,
    0,
    1,
    1,
    1
  ]);
  var s_renderTargetReverseUvArray = new Float32Array([
    0,
    1,
    1,
    1,
    0,
    0,
    1,
    0
  ]);
  var CubismShader_WebGL = class {
    /**
     * 非同期でシェーダーをパスから読み込む
     *
     * @param url シェーダーのURL
     *
     * @return シェーダーのソースコード
     */
    async loadShader(url) {
      return await cachedFetchText(url);
    }
    /**
     * ブレンドモード用のシェーダーを読み込む
     */
    async loadShaders() {
      var _a;
      const shaderDir = (_a = this._shaderPath) != null ? _a : this._defaultShaderPath;
      const shaderFiles = [
        { path: shaderDir + VertShaderSrcPath, prop: "_vertShaderSrc" },
        {
          path: shaderDir + VertShaderSrcMaskedPath,
          prop: "_vertShaderSrcMasked"
        },
        {
          path: shaderDir + VertShaderSrcSetupMaskPath,
          prop: "_vertShaderSrcSetupMask"
        },
        {
          path: shaderDir + FragShaderSrcSetupMaskPath,
          prop: "_fragShaderSrcSetupMask"
        },
        {
          path: shaderDir + FragShaderSrcPremultipliedAlphaPath,
          prop: "_fragShaderSrcPremultipliedAlpha"
        },
        {
          path: shaderDir + FragShaderSrcMaskPremultipliedAlphaPath,
          prop: "_fragShaderSrcMaskPremultipliedAlpha"
        },
        {
          path: shaderDir + FragShaderSrcMaskInvertedPremultipliedAlphaPath,
          prop: "_fragShaderSrcMaskInvertedPremultipliedAlpha"
        },
        { path: shaderDir + VertShaderSrcCopyPath, prop: "_vertShaderSrcCopy" },
        { path: shaderDir + FragShaderSrcCopyPath, prop: "_fragShaderSrcCopy" },
        {
          path: shaderDir + FragShaderSrcColorBlendPath,
          prop: "_fragShaderSrcColorBlend"
        },
        {
          path: shaderDir + FragShaderSrcAlphaBlendPath,
          prop: "_fragShaderSrcAlphaBlend"
        },
        { path: shaderDir + VertShaderSrcBlendPath, prop: "_vertShaderSrcBlend" },
        { path: shaderDir + FragShaderSrcBlendPath, prop: "_fragShaderSrcBlend" }
      ];
      const results = await Promise.all(
        shaderFiles.map(
          (file) => this.loadShader(file.path).then((data) => ({ prop: file.prop, data })).catch((error) => {
            console.error(`Error loading ${file.path} shader:`, error);
            return { prop: file.prop, data: "" };
          })
        )
      );
      results.forEach((result) => {
        this[result.prop] = result.data;
      });
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._shaderSets = new Array();
      this._isShaderLoading = false;
      this._isShaderLoaded = false;
      this._colorBlendMap = /* @__PURE__ */ new Map();
      this._colorBlendValues = new Array();
      const colorBlendKeys = Object.keys(CubismColorBlend);
      const colorBlendRawValues = Object.keys(CubismColorBlend).map(
        (k) => CubismColorBlend[k]
      );
      for (let i = 0; i < colorBlendKeys.length; i++) {
        const colorBlendKey = colorBlendKeys[i];
        if (colorBlendKey.includes(ColorBlendPrefix)) {
          const blendModeName = colorBlendKey.slice(ColorBlendPrefix.length);
          const colorBlendNumber = parseInt(colorBlendRawValues[i].toString());
          this._colorBlendMap.set(colorBlendNumber, blendModeName);
          this._colorBlendValues.push(colorBlendNumber);
        }
      }
      this._alphaBlendMap = /* @__PURE__ */ new Map();
      this._alphaBlendValues = new Array();
      const alphaBlendKeys = Object.keys(CubismAlphaBlend);
      const alphaBlendRawValues = Object.keys(CubismAlphaBlend).map(
        (k) => CubismAlphaBlend[k]
      );
      for (let i = 0; i < alphaBlendKeys.length; i++) {
        const alphaBlendKey = alphaBlendKeys[i];
        if (alphaBlendKey.includes(AlphaBlendPrefix)) {
          const blendModeName = alphaBlendKey.slice(AlphaBlendPrefix.length);
          const alphaBlendNumber = parseInt(alphaBlendRawValues[i].toString());
          this._alphaBlendMap.set(alphaBlendNumber, blendModeName);
          this._alphaBlendValues.push(alphaBlendNumber);
        }
      }
      this._blendShaderSetMap = /* @__PURE__ */ new Map();
      this._shaderCount = 10 /* ShaderNames_ShaderCount */ + 1 + (this._colorBlendValues.length - 3) * (this._alphaBlendValues.length - 1) * 3;
      this._defaultShaderPath = "/resources/live2d/shaders/";
      this._shaderPath = this._defaultShaderPath;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this.releaseShaderProgram();
    }
    /**
     * 描画用のシェーダプログラムの一連のセットアップを実行する
     *
     * @param renderer レンダラー
     * @param model 描画対象のモデル
     * @param index 描画対象のメッシュのインデックス
     */
    setupShaderProgramForDrawable(renderer, model, index) {
      if (!renderer.isPremultipliedAlpha()) {
        CubismLogError("NoPremultipliedAlpha is not allowed");
      }
      if (this._shaderSets.length == 0) {
        this.generateShaders();
      }
      if (this._isShaderLoaded == false) {
        CubismLogWarning("Shader program is not initialized.");
        return;
      }
      let srcColor;
      let dstColor;
      let srcAlpha;
      let dstAlpha;
      const masked = renderer.getClippingContextBufferForDrawable() != null;
      const invertedMask = model.getDrawableInvertedMaskBit(index);
      const offset = masked ? invertedMask ? 2 : 1 : 0;
      let shaderSet;
      let isUsingCompatible = true;
      if (model.isBlendModeEnabled()) {
        const colorBlendMode = model.getDrawableColorBlend(index);
        const alphaBlendMode = model.getDrawableAlphaBlend(index);
        if (colorBlendMode == -1 /* ColorBlend_None */ || alphaBlendMode == -1 /* AlphaBlend_None */ || colorBlendMode == CubismColorBlend.ColorBlend_Normal && alphaBlendMode == 0 /* AlphaBlend_Over */) {
          shaderSet = this._shaderSets[1 /* ShaderNames_NormalPremultipliedAlpha */ + offset];
          srcColor = this.gl.ONE;
          dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
          srcAlpha = this.gl.ONE;
          dstAlpha = this.gl.ONE_MINUS_SRC_ALPHA;
        } else {
          switch (colorBlendMode) {
            // Cubism 5.2以前のシェーダを使用する。
            case CubismColorBlend.ColorBlend_AddCompatible:
              shaderSet = this._shaderSets[4 /* ShaderNames_AddPremultipliedAlpha */ + offset];
              srcColor = this.gl.ONE;
              dstColor = this.gl.ONE;
              srcAlpha = this.gl.ZERO;
              dstAlpha = this.gl.ONE;
              break;
            // Cubism 5.2以前のシェーダを使用する。
            case CubismColorBlend.ColorBlend_MultiplyCompatible:
              shaderSet = this._shaderSets[7 /* ShaderNames_MultPremultipliedAlpha */ + offset];
              srcColor = this.gl.DST_COLOR;
              dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
              srcAlpha = this.gl.ZERO;
              dstAlpha = this.gl.ONE;
              break;
            // ブレンドモードの組み合わせでシェーダーを決定
            default:
              {
                const srcBuffer = renderer._currentOffscreen != null ? renderer._currentOffscreen : renderer.getModelRenderTarget(0);
                CubismRenderTarget_WebGL.copyBuffer(
                  this.gl,
                  srcBuffer,
                  renderer.getModelRenderTarget(1)
                );
                const baseShaderSetIndex = this._blendShaderSetMap.get(
                  this._colorBlendMap.get(colorBlendMode) + this._alphaBlendMap.get(alphaBlendMode)
                );
                shaderSet = this._shaderSets[baseShaderSetIndex + offset];
                srcColor = this.gl.ONE;
                dstColor = this.gl.ZERO;
                srcAlpha = this.gl.ONE;
                dstAlpha = this.gl.ZERO;
                isUsingCompatible = false;
              }
              break;
          }
        }
      } else {
        switch (model.getDrawableBlendMode(index)) {
          case 0 /* CubismBlendMode_Normal */:
          default:
            shaderSet = this._shaderSets[1 /* ShaderNames_NormalPremultipliedAlpha */ + offset];
            srcColor = this.gl.ONE;
            dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
            srcAlpha = this.gl.ONE;
            dstAlpha = this.gl.ONE_MINUS_SRC_ALPHA;
            break;
          case 1 /* CubismBlendMode_Additive */:
            shaderSet = this._shaderSets[4 /* ShaderNames_AddPremultipliedAlpha */ + offset];
            srcColor = this.gl.ONE;
            dstColor = this.gl.ONE;
            srcAlpha = this.gl.ZERO;
            dstAlpha = this.gl.ONE;
            break;
          case 2 /* CubismBlendMode_Multiplicative */:
            shaderSet = this._shaderSets[7 /* ShaderNames_MultPremultipliedAlpha */ + offset];
            srcColor = this.gl.DST_COLOR;
            dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
            srcAlpha = this.gl.ZERO;
            dstAlpha = this.gl.ONE;
            break;
        }
      }
      this.gl.useProgram(shaderSet.shaderProgram);
      if (renderer._bufferData.vertex == null) {
        renderer._bufferData.vertex = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.vertex);
      const vertexArray = model.getDrawableVertices(index);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, vertexArray, this.gl.DYNAMIC_DRAW);
      this.gl.enableVertexAttribArray(shaderSet.attributePositionLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributePositionLocation,
        2,
        this.gl.FLOAT,
        false,
        0,
        0
      );
      if (renderer._bufferData.uv == null) {
        renderer._bufferData.uv = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.uv);
      const uvArray = model.getDrawableVertexUvs(index);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, uvArray, this.gl.DYNAMIC_DRAW);
      this.gl.enableVertexAttribArray(shaderSet.attributeTexCoordLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributeTexCoordLocation,
        2,
        this.gl.FLOAT,
        false,
        0,
        0
      );
      if (masked) {
        this.gl.activeTexture(this.gl.TEXTURE1);
        const tex = renderer.getDrawableMaskBuffer(
          renderer.getClippingContextBufferForDrawable()._bufferIndex
        ).getColorBuffer();
        this.gl.bindTexture(this.gl.TEXTURE_2D, tex);
        this.gl.uniform1i(shaderSet.samplerTexture1Location, 1);
        this.gl.uniformMatrix4fv(
          shaderSet.uniformClipMatrixLocation,
          false,
          renderer.getClippingContextBufferForDrawable()._matrixForDraw.getArray()
        );
        const channelIndex = renderer.getClippingContextBufferForDrawable()._layoutChannelIndex;
        const colorChannel = renderer.getClippingContextBufferForDrawable().getClippingManager().getChannelFlagAsColor(channelIndex);
        this.gl.uniform4f(
          shaderSet.uniformChannelFlagLocation,
          colorChannel.r,
          colorChannel.g,
          colorChannel.b,
          colorChannel.a
        );
        if (model.isBlendModeEnabled()) {
          this.gl.uniform1f(
            shaderSet.uniformInvertMaskFlagLocation,
            invertedMask ? 1 : 0
          );
        }
      }
      const textureNo = model.getDrawableTextureIndex(index);
      const textureId = renderer.getBindedTextures().get(textureNo);
      this.gl.activeTexture(this.gl.TEXTURE0);
      this.gl.bindTexture(this.gl.TEXTURE_2D, textureId);
      this.gl.uniform1i(shaderSet.samplerTexture0Location, 0);
      const matrix4x4 = renderer.getMvpMatrix();
      this.gl.uniformMatrix4fv(
        shaderSet.uniformMatrixLocation,
        false,
        matrix4x4.getArray()
      );
      let baseColor = null;
      if (model.isBlendModeEnabled()) {
        const drawableOpacity = model.getDrawableOpacity(index);
        baseColor = new CubismTextureColor(
          drawableOpacity,
          drawableOpacity,
          drawableOpacity,
          drawableOpacity
        );
      } else {
        baseColor = renderer.getModelColorWithOpacity(
          model.getDrawableOpacity(index)
        );
      }
      const multiplyAndScreenColor = model.getOverrideMultiplyAndScreenColor();
      const multiplyColor = multiplyAndScreenColor.getDrawableMultiplyColor(index);
      const screenColor = multiplyAndScreenColor.getDrawableScreenColor(index);
      this.gl.uniform4f(
        shaderSet.uniformBaseColorLocation,
        baseColor.r,
        baseColor.g,
        baseColor.b,
        baseColor.a
      );
      this.gl.uniform4f(
        shaderSet.uniformMultiplyColorLocation,
        multiplyColor.r,
        multiplyColor.g,
        multiplyColor.b,
        multiplyColor.a
      );
      this.gl.uniform4f(
        shaderSet.uniformScreenColorLocation,
        screenColor.r,
        screenColor.g,
        screenColor.b,
        screenColor.a
      );
      if (model.isBlendModeEnabled()) {
        this.gl.activeTexture(this.gl.TEXTURE2);
        if (!isUsingCompatible) {
          const tex = renderer.getModelRenderTarget(1).getColorBuffer();
          this.gl.bindTexture(this.gl.TEXTURE_2D, tex);
          this.gl.uniform1i(shaderSet.samplerFrameBufferTextureLocation, 2);
        }
      }
      if (renderer._bufferData.index == null) {
        renderer._bufferData.index = this.gl.createBuffer();
      }
      const indexArray = model.getDrawableVertexIndices(index);
      this.gl.bindBuffer(
        this.gl.ELEMENT_ARRAY_BUFFER,
        renderer._bufferData.index
      );
      this.gl.bufferData(
        this.gl.ELEMENT_ARRAY_BUFFER,
        indexArray,
        this.gl.DYNAMIC_DRAW
      );
      this.gl.blendFuncSeparate(srcColor, dstColor, srcAlpha, dstAlpha);
    }
    /**
     * オフスクリーン用のシェーダプログラムの一連のセットアップを実行する
     *
     * @param renderer レンダラー
     * @param model 描画対象のモデル
     * @param offscreen 描画対象のオフスクリーン
     */
    setupShaderProgramForOffscreen(renderer, model, offscreen) {
      if (!renderer.isPremultipliedAlpha()) {
        CubismLogError("NoPremultipliedAlpha is not allowed");
      }
      if (this._shaderSets.length == 0) {
        this.generateShaders();
      }
      if (this._isShaderLoaded == false) {
        CubismLogWarning("Shader program is not initialized.");
        return;
      }
      let srcColor;
      let dstColor;
      let srcAlpha;
      let dstAlpha;
      const offscreenIndex = offscreen.getOffscreenIndex();
      const masked = renderer.getClippingContextBufferForOffscreen() != null;
      const invertedMask = model.getOffscreenInvertedMask(offscreenIndex);
      const offset = masked ? invertedMask ? 2 : 1 : 0;
      let shaderSet;
      let isUsingCompatible = true;
      const colorBlendMode = model.getOffscreenColorBlend(offscreenIndex);
      const alphaBlendMode = model.getOffscreenAlphaBlend(offscreenIndex);
      if (colorBlendMode == -1 /* ColorBlend_None */ || alphaBlendMode == -1 /* AlphaBlend_None */ || colorBlendMode == CubismColorBlend.ColorBlend_Normal && alphaBlendMode == 0 /* AlphaBlend_Over */) {
        shaderSet = this._shaderSets[1 /* ShaderNames_NormalPremultipliedAlpha */ + offset];
        srcColor = this.gl.ONE;
        dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
        srcAlpha = this.gl.ONE;
        dstAlpha = this.gl.ONE_MINUS_SRC_ALPHA;
      } else {
        switch (colorBlendMode) {
          // Cubism 5.2以前のシェーダを使用する。
          case CubismColorBlend.ColorBlend_AddCompatible:
            shaderSet = this._shaderSets[4 /* ShaderNames_AddPremultipliedAlpha */ + offset];
            srcColor = this.gl.ONE;
            dstColor = this.gl.ONE;
            srcAlpha = this.gl.ZERO;
            dstAlpha = this.gl.ONE;
            break;
          case CubismColorBlend.ColorBlend_MultiplyCompatible:
            shaderSet = this._shaderSets[7 /* ShaderNames_MultPremultipliedAlpha */ + offset];
            srcColor = this.gl.DST_COLOR;
            dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
            srcAlpha = this.gl.ZERO;
            dstAlpha = this.gl.ONE;
            break;
          default:
            {
              const srcBuffer = offscreen.getOldOffscreen() != null ? offscreen.getOldOffscreen() : renderer.getModelRenderTarget(0);
              CubismRenderTarget_WebGL.copyBuffer(
                this.gl,
                srcBuffer,
                renderer.getModelRenderTarget(1)
              );
              const baseShaderSetIndex = this._blendShaderSetMap.get(
                this._colorBlendMap.get(colorBlendMode) + this._alphaBlendMap.get(alphaBlendMode)
              );
              shaderSet = this._shaderSets[baseShaderSetIndex + offset];
              srcColor = this.gl.ONE;
              dstColor = this.gl.ZERO;
              srcAlpha = this.gl.ONE;
              dstAlpha = this.gl.ZERO;
              isUsingCompatible = false;
            }
            break;
        }
      }
      this.gl.useProgram(shaderSet.shaderProgram);
      CubismRenderTarget_WebGL.copyBuffer(
        this.gl,
        offscreen,
        renderer.getModelRenderTarget(2)
      );
      this.gl.activeTexture(this.gl.TEXTURE0);
      const tex0 = renderer.getModelRenderTarget(2).getColorBuffer();
      this.gl.bindTexture(this.gl.TEXTURE_2D, tex0);
      this.gl.uniform1i(shaderSet.samplerTexture0Location, 0);
      const matrix4x4 = new CubismMatrix44();
      matrix4x4.loadIdentity();
      this.gl.uniformMatrix4fv(
        shaderSet.uniformMatrixLocation,
        false,
        matrix4x4.getArray()
      );
      const offscreenOpacity = model.getOffscreenOpacity(offscreenIndex);
      const baseColor = new CubismTextureColor(
        offscreenOpacity,
        offscreenOpacity,
        offscreenOpacity,
        offscreenOpacity
      );
      const multiplyAndScreenColor = model.getOverrideMultiplyAndScreenColor();
      const multiplyColor = multiplyAndScreenColor.getOffscreenMultiplyColor(offscreenIndex);
      const screenColor = multiplyAndScreenColor.getOffscreenScreenColor(offscreenIndex);
      this.gl.uniform4f(
        shaderSet.uniformBaseColorLocation,
        baseColor.r,
        baseColor.g,
        baseColor.b,
        baseColor.a
      );
      this.gl.uniform4f(
        shaderSet.uniformMultiplyColorLocation,
        multiplyColor.r,
        multiplyColor.g,
        multiplyColor.b,
        multiplyColor.a
      );
      this.gl.uniform4f(
        shaderSet.uniformScreenColorLocation,
        screenColor.r,
        screenColor.g,
        screenColor.b,
        screenColor.a
      );
      this.gl.activeTexture(this.gl.TEXTURE2);
      if (!isUsingCompatible) {
        const tex1 = renderer.getModelRenderTarget(1).getColorBuffer();
        this.gl.bindTexture(this.gl.TEXTURE_2D, tex1);
        this.gl.uniform1i(shaderSet.samplerFrameBufferTextureLocation, 2);
      }
      if (masked) {
        this.gl.activeTexture(this.gl.TEXTURE1);
        const tex2 = renderer.getOffscreenMaskBuffer(
          renderer.getClippingContextBufferForOffscreen()._bufferIndex
        ).getColorBuffer();
        this.gl.bindTexture(this.gl.TEXTURE_2D, tex2);
        this.gl.uniform1i(shaderSet.samplerTexture1Location, 1);
        this.gl.uniformMatrix4fv(
          shaderSet.uniformClipMatrixLocation,
          false,
          renderer.getClippingContextBufferForOffscreen()._matrixForDraw.getArray()
        );
        const channelIndex = renderer.getClippingContextBufferForOffscreen()._layoutChannelIndex;
        const colorChannel = renderer.getClippingContextBufferForOffscreen().getClippingManager().getChannelFlagAsColor(channelIndex);
        this.gl.uniform4f(
          shaderSet.uniformChannelFlagLocation,
          colorChannel.r,
          colorChannel.g,
          colorChannel.b,
          colorChannel.a
        );
        if (model.isBlendModeEnabled()) {
          this.gl.uniform1f(
            shaderSet.uniformInvertMaskFlagLocation,
            invertedMask ? 1 : 0
          );
        }
      }
      if (!renderer._bufferData.vertex) {
        renderer._bufferData.vertex = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.vertex);
      this.gl.bufferData(
        this.gl.ARRAY_BUFFER,
        s_renderTargetVertexArray,
        this.gl.STATIC_DRAW
      );
      this.gl.enableVertexAttribArray(shaderSet.attributePositionLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributePositionLocation,
        2,
        this.gl.FLOAT,
        false,
        Float32Array.BYTES_PER_ELEMENT * 2,
        0
      );
      if (!renderer._bufferData.uv) {
        renderer._bufferData.uv = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.uv);
      this.gl.bufferData(
        this.gl.ARRAY_BUFFER,
        s_renderTargetReverseUvArray,
        this.gl.STATIC_DRAW
      );
      this.gl.enableVertexAttribArray(shaderSet.attributeTexCoordLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributeTexCoordLocation,
        2,
        this.gl.FLOAT,
        false,
        Float32Array.BYTES_PER_ELEMENT * 2,
        0
      );
      this.gl.blendFuncSeparate(srcColor, dstColor, srcAlpha, dstAlpha);
    }
    /**
     * マスク用のシェーダプログラムの一連のセットアップを実行する
     *
     * @param renderer レンダラー
     * @param model 描画対象のモデル
     * @param index 描画対象のメッシュのインデックス
     */
    setupShaderProgramForMask(renderer, model, index) {
      if (!renderer.isPremultipliedAlpha()) {
        CubismLogError("NoPremultipliedAlpha is not allowed");
      }
      if (this._shaderSets.length == 0) {
        this.generateShaders();
      }
      if (this._isShaderLoaded == false) {
        CubismLogWarning("Shader program is not initialized.");
        return;
      }
      const shaderSet = this._shaderSets[0 /* ShaderNames_SetupMask */];
      this.gl.useProgram(shaderSet.shaderProgram);
      if (renderer._bufferData.vertex == null) {
        renderer._bufferData.vertex = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.vertex);
      const vertexArray = model.getDrawableVertices(index);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, vertexArray, this.gl.DYNAMIC_DRAW);
      this.gl.enableVertexAttribArray(shaderSet.attributePositionLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributePositionLocation,
        2,
        this.gl.FLOAT,
        false,
        0,
        0
      );
      if (renderer._bufferData.uv == null) {
        renderer._bufferData.uv = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.uv);
      const textureNo = model.getDrawableTextureIndex(index);
      const textureId = renderer.getBindedTextures().get(textureNo);
      this.gl.activeTexture(this.gl.TEXTURE0);
      this.gl.bindTexture(this.gl.TEXTURE_2D, textureId);
      this.gl.uniform1i(shaderSet.samplerTexture0Location, 0);
      if (renderer._bufferData.uv == null) {
        renderer._bufferData.uv = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.uv);
      const uvArray = model.getDrawableVertexUvs(index);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, uvArray, this.gl.DYNAMIC_DRAW);
      this.gl.enableVertexAttribArray(shaderSet.attributeTexCoordLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributeTexCoordLocation,
        2,
        this.gl.FLOAT,
        false,
        0,
        0
      );
      const channelIndex = renderer.getClippingContextBufferForMask()._layoutChannelIndex;
      const colorChannel = renderer.getClippingContextBufferForMask().getClippingManager().getChannelFlagAsColor(channelIndex);
      this.gl.uniform4f(
        shaderSet.uniformChannelFlagLocation,
        colorChannel.r,
        colorChannel.g,
        colorChannel.b,
        colorChannel.a
      );
      this.gl.uniformMatrix4fv(
        shaderSet.uniformClipMatrixLocation,
        false,
        renderer.getClippingContextBufferForMask()._matrixForMask.getArray()
      );
      const rect = renderer.getClippingContextBufferForMask()._layoutBounds;
      this.gl.uniform4f(
        shaderSet.uniformBaseColorLocation,
        rect.x * 2 - 1,
        rect.y * 2 - 1,
        rect.getRight() * 2 - 1,
        rect.getBottom() * 2 - 1
      );
      const srcColor = this.gl.ZERO;
      const dstColor = this.gl.ONE_MINUS_SRC_COLOR;
      const srcAlpha = this.gl.ZERO;
      const dstAlpha = this.gl.ONE_MINUS_SRC_ALPHA;
      if (renderer._bufferData.index == null) {
        renderer._bufferData.index = this.gl.createBuffer();
      }
      const indexArray = model.getDrawableVertexIndices(index);
      this.gl.bindBuffer(
        this.gl.ELEMENT_ARRAY_BUFFER,
        renderer._bufferData.index
      );
      this.gl.bufferData(
        this.gl.ELEMENT_ARRAY_BUFFER,
        indexArray,
        this.gl.DYNAMIC_DRAW
      );
      this.gl.blendFuncSeparate(srcColor, dstColor, srcAlpha, dstAlpha);
    }
    /**
     * オフスクリーンのレンダリングターゲット用のシェーダープログラムを設定する
     *
     * @param renderer レンダラー
     */
    setupShaderProgramForOffscreenRenderTarget(renderer) {
      if (this._shaderSets.length == 0) {
        this.generateShaders();
      }
      if (this._isShaderLoaded == false) {
        CubismLogWarning("Shader program is not initialized.");
        return;
      }
      const baseColor = renderer.getModelColor();
      baseColor.r *= baseColor.a;
      baseColor.g *= baseColor.a;
      baseColor.b *= baseColor.a;
      this.copyTexture(renderer, baseColor);
    }
    /**
     * オフスクリーンのレンダリングターゲットの内容をコピーする
     *
     * @param renderer レンダラー
     * @param baseColor ベースカラー
     */
    copyTexture(renderer, baseColor) {
      const srcColor = this.gl.ONE;
      const dstColor = this.gl.ONE_MINUS_SRC_ALPHA;
      const srcAlpha = this.gl.ONE;
      const dstAlpha = this.gl.ONE_MINUS_SRC_ALPHA;
      const shaderSet = this._shaderSets[10];
      this.gl.useProgram(shaderSet.shaderProgram);
      this.gl.uniform4f(
        shaderSet.uniformBaseColorLocation,
        baseColor.r,
        baseColor.g,
        baseColor.b,
        baseColor.a
      );
      this.gl.activeTexture(this.gl.TEXTURE0);
      const tex = renderer.getModelRenderTarget(0).getColorBuffer();
      this.gl.bindTexture(this.gl.TEXTURE_2D, tex);
      this.gl.uniform1i(shaderSet.samplerTexture0Location, 0);
      if (!renderer._bufferData.vertex) {
        renderer._bufferData.vertex = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.vertex);
      this.gl.bufferData(
        this.gl.ARRAY_BUFFER,
        s_renderTargetVertexArray,
        this.gl.STATIC_DRAW
      );
      this.gl.enableVertexAttribArray(shaderSet.attributePositionLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributePositionLocation,
        2,
        this.gl.FLOAT,
        false,
        Float32Array.BYTES_PER_ELEMENT * 2,
        0
      );
      if (!renderer._bufferData.uv) {
        renderer._bufferData.uv = this.gl.createBuffer();
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, renderer._bufferData.uv);
      this.gl.bufferData(
        this.gl.ARRAY_BUFFER,
        s_renderTargetUvArray,
        this.gl.STATIC_DRAW
      );
      this.gl.enableVertexAttribArray(shaderSet.attributeTexCoordLocation);
      this.gl.vertexAttribPointer(
        shaderSet.attributeTexCoordLocation,
        2,
        this.gl.FLOAT,
        false,
        Float32Array.BYTES_PER_ELEMENT * 2,
        0
      );
      this.gl.blendFuncSeparate(srcColor, dstColor, srcAlpha, dstAlpha);
    }
    /**
     * シェーダープログラムを解放する
     */
    releaseShaderProgram() {
      for (let i = 0; i < this._shaderSets.length; i++) {
        this.gl.deleteProgram(this._shaderSets[i].shaderProgram);
        this._shaderSets[i].shaderProgram = 0;
        this._shaderSets[i] = void 0;
        this._shaderSets[i] = null;
      }
    }
    /**
     * シェーダープログラムを初期化する
     *
     * @param vertShaderSrc 頂点シェーダのソース
     * @param fragShaderSrc フラグメントシェーダのソース
     */
    generateShaders() {
      if (this._isShaderLoading) {
        return;
      }
      this._isShaderLoading = true;
      this._isShaderLoaded = false;
      this._shaderSets.length = this._shaderCount;
      for (let i = 0; i < this._shaderCount; i++) {
        this._shaderSets[i] = new CubismShaderSet();
      }
      this.loadShaders().then(() => {
        this.registerShader();
        this.registerBlendShader();
        this._isShaderLoading = false;
        this._isShaderLoaded = true;
      }).catch((error) => {
        this._isShaderLoading = false;
        console.error("Failed to load shaders:", error);
      });
    }
    /**
     * シェーダープログラムを登録する
     */
    registerShader() {
      const vertexShaderSrc = this._vertShaderSrc;
      const vertexShaderSrcMasked = this._vertShaderSrcMasked;
      const vertexShaderSrcSetupMask = this._vertShaderSrcSetupMask;
      const fragmentShaderSrcSetupMask = this._fragShaderSrcSetupMask;
      const fragmentShaderSrcPremultipliedAlpha = this._fragShaderSrcPremultipliedAlpha;
      const fragmentShaderSrcMaskPremultipliedAlpha = this._fragShaderSrcMaskPremultipliedAlpha;
      const fragmentShaderSrcMaskInvertedPremultipliedAlpha = this._fragShaderSrcMaskInvertedPremultipliedAlpha;
      this._shaderSets[0].shaderProgram = this.loadShaderProgram(
        vertexShaderSrcSetupMask,
        fragmentShaderSrcSetupMask
      );
      this._shaderSets[1].shaderProgram = this.loadShaderProgram(
        vertexShaderSrc,
        fragmentShaderSrcPremultipliedAlpha
      );
      this._shaderSets[2].shaderProgram = this.loadShaderProgram(
        vertexShaderSrcMasked,
        fragmentShaderSrcMaskPremultipliedAlpha
      );
      this._shaderSets[3].shaderProgram = this.loadShaderProgram(
        vertexShaderSrcMasked,
        fragmentShaderSrcMaskInvertedPremultipliedAlpha
      );
      this._shaderSets[4].shaderProgram = this._shaderSets[1].shaderProgram;
      this._shaderSets[5].shaderProgram = this._shaderSets[2].shaderProgram;
      this._shaderSets[6].shaderProgram = this._shaderSets[3].shaderProgram;
      this._shaderSets[7].shaderProgram = this._shaderSets[1].shaderProgram;
      this._shaderSets[8].shaderProgram = this._shaderSets[2].shaderProgram;
      this._shaderSets[9].shaderProgram = this._shaderSets[3].shaderProgram;
      this._shaderSets[0].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[0].shaderProgram,
        "a_position"
      );
      this._shaderSets[0].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[0].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[0].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[0].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[0].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[0].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[0].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[0].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[0].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[0].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[1].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[1].shaderProgram,
        "a_position"
      );
      this._shaderSets[1].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[1].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[1].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[1].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[1].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[1].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[1].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[1].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[1].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[1].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[1].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[1].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[2].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[2].shaderProgram,
        "a_position"
      );
      this._shaderSets[2].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[2].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[2].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[2].samplerTexture1Location = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "s_texture1"
      );
      this._shaderSets[2].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[2].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[2].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[2].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[2].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[2].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[2].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[3].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[3].shaderProgram,
        "a_position"
      );
      this._shaderSets[3].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[3].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[3].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[3].samplerTexture1Location = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "s_texture1"
      );
      this._shaderSets[3].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[3].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[3].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[3].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[3].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[3].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[3].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[4].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[4].shaderProgram,
        "a_position"
      );
      this._shaderSets[4].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[4].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[4].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[4].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[4].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[4].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[4].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[4].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[4].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[4].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[4].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[4].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[5].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[5].shaderProgram,
        "a_position"
      );
      this._shaderSets[5].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[5].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[5].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[5].samplerTexture1Location = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "s_texture1"
      );
      this._shaderSets[5].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[5].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[5].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[5].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[5].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[5].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[5].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[6].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[6].shaderProgram,
        "a_position"
      );
      this._shaderSets[6].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[6].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[6].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[6].samplerTexture1Location = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "s_texture1"
      );
      this._shaderSets[6].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[6].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[6].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[6].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[6].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[6].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[6].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[7].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[7].shaderProgram,
        "a_position"
      );
      this._shaderSets[7].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[7].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[7].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[7].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[7].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[7].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[7].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[7].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[7].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[7].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[7].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[7].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[8].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[8].shaderProgram,
        "a_position"
      );
      this._shaderSets[8].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[8].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[8].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[8].samplerTexture1Location = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "s_texture1"
      );
      this._shaderSets[8].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[8].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[8].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[8].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[8].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[8].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[8].shaderProgram,
        "u_screenColor"
      );
      this._shaderSets[9].attributePositionLocation = this.gl.getAttribLocation(
        this._shaderSets[9].shaderProgram,
        "a_position"
      );
      this._shaderSets[9].attributeTexCoordLocation = this.gl.getAttribLocation(
        this._shaderSets[9].shaderProgram,
        "a_texCoord"
      );
      this._shaderSets[9].samplerTexture0Location = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "s_texture0"
      );
      this._shaderSets[9].samplerTexture1Location = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "s_texture1"
      );
      this._shaderSets[9].uniformMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "u_matrix"
      );
      this._shaderSets[9].uniformClipMatrixLocation = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "u_clipMatrix"
      );
      this._shaderSets[9].uniformChannelFlagLocation = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "u_channelFlag"
      );
      this._shaderSets[9].uniformBaseColorLocation = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "u_baseColor"
      );
      this._shaderSets[9].uniformMultiplyColorLocation = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "u_multiplyColor"
      );
      this._shaderSets[9].uniformScreenColorLocation = this.gl.getUniformLocation(
        this._shaderSets[9].shaderProgram,
        "u_screenColor"
      );
    }
    /**
     * ブレンドモード用のシェーダープログラムを登録する
     */
    registerBlendShader() {
      const vertShaderSrcCopy = this._vertShaderSrcCopy;
      const fragShaderSrcCopy = this._fragShaderSrcCopy;
      const copyShaderSet = this._shaderSets[10];
      copyShaderSet.shaderProgram = this.loadShaderProgram(
        vertShaderSrcCopy,
        fragShaderSrcCopy
      );
      copyShaderSet.attributeTexCoordLocation = this.gl.getAttribLocation(
        copyShaderSet.shaderProgram,
        "a_texCoord"
      );
      copyShaderSet.attributePositionLocation = this.gl.getAttribLocation(
        copyShaderSet.shaderProgram,
        "a_position"
      );
      copyShaderSet.uniformBaseColorLocation = this.gl.getUniformLocation(
        copyShaderSet.shaderProgram,
        "u_baseColor"
      );
      let shaderSetIndex = 11;
      for (let colorBlendIndex = 0; colorBlendIndex < this._colorBlendValues.length; colorBlendIndex++) {
        if (this._colorBlendValues[colorBlendIndex] == -1 /* ColorBlend_None */ || this._colorBlendValues[colorBlendIndex] == CubismColorBlend.ColorBlend_AddCompatible || this._colorBlendValues[colorBlendIndex] == CubismColorBlend.ColorBlend_MultiplyCompatible) {
          continue;
        }
        const colorBlendValue = this._colorBlendValues[colorBlendIndex];
        const colorBlendName = this._colorBlendMap.get(colorBlendValue).toUpperCase();
        const colorBlendMacro = `#define COLOR_BLEND_${colorBlendName}
`;
        for (let alphablendIndex = 0; alphablendIndex < this._alphaBlendValues.length; alphablendIndex++) {
          if (this._alphaBlendValues[alphablendIndex] == -1 /* AlphaBlend_None */ || this._colorBlendValues[colorBlendIndex] == CubismColorBlend.ColorBlend_Normal && this._alphaBlendValues[alphablendIndex] == 0 /* AlphaBlend_Over */) {
            continue;
          }
          const alphaBlendValue = this._alphaBlendValues[alphablendIndex];
          const alphaBlendName = this._alphaBlendMap.get(alphaBlendValue).toUpperCase();
          const alphaBlendMacro = `#define ALPHA_BLEND_${alphaBlendName}
`;
          this.generateBlendShader(
            colorBlendMacro,
            alphaBlendMacro,
            shaderSetIndex
          );
          this._blendShaderSetMap.set(
            this._colorBlendMap.get(this._colorBlendValues[colorBlendIndex]) + this._alphaBlendMap.get(this._alphaBlendValues[alphablendIndex]),
            shaderSetIndex
          );
          shaderSetIndex += 3 /* ShaderType_Count */;
        }
      }
    }
    /**
     * ブレンドモード用のシェーダープログラムを生成する
     *
     * @param colorBlendMacro カラーブレンド用のマクロ
     * @param alphaBlendMacro アルファブレンド用のマクロ
     * @param shaderSetBaseIndex _shaderSets のインデックス
     */
    generateBlendShader(colorBlendMacro, alphaBlendMacro, shaderSetBaseIndex) {
      for (let shaderTypeIndex = 0; shaderTypeIndex < 3 /* ShaderType_Count */; shaderTypeIndex++) {
        let vertexShaderSrc = "";
        let fragmentShaderStr = "precision mediump float;\n";
        const shaderSetIndex = shaderSetBaseIndex + shaderTypeIndex;
        fragmentShaderStr += colorBlendMacro;
        fragmentShaderStr += alphaBlendMacro;
        fragmentShaderStr += this._fragShaderSrcColorBlend;
        fragmentShaderStr += this._fragShaderSrcAlphaBlend;
        if (shaderTypeIndex == 1 /* ShaderType_Masked */ || shaderTypeIndex == 2 /* ShaderType_MaskedInverted */) {
          const clippingMaskMacro = "#define CLIPPING_MASK\n";
          vertexShaderSrc += clippingMaskMacro;
          fragmentShaderStr += clippingMaskMacro;
        }
        vertexShaderSrc += this._vertShaderSrcBlend;
        fragmentShaderStr += this._fragShaderSrcBlend;
        this._shaderSets[shaderSetIndex].shaderProgram = this.loadShaderProgram(
          vertexShaderSrc,
          fragmentShaderStr
        );
        this._shaderSets[shaderSetIndex].attributePositionLocation = this.gl.getAttribLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "a_position"
        );
        this._shaderSets[shaderSetIndex].attributeTexCoordLocation = this.gl.getAttribLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "a_texCoord"
        );
        this._shaderSets[shaderSetIndex].samplerTexture0Location = this.gl.getUniformLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "s_texture0"
        );
        this._shaderSets[shaderSetIndex].uniformMatrixLocation = this.gl.getUniformLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "u_matrix"
        );
        this._shaderSets[shaderSetIndex].uniformBaseColorLocation = this.gl.getUniformLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "u_baseColor"
        );
        this._shaderSets[shaderSetIndex].uniformMultiplyColorLocation = this.gl.getUniformLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "u_multiplyColor"
        );
        this._shaderSets[shaderSetIndex].uniformScreenColorLocation = this.gl.getUniformLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "u_screenColor"
        );
        this._shaderSets[shaderSetIndex].samplerFrameBufferTextureLocation = this.gl.getUniformLocation(
          this._shaderSets[shaderSetIndex].shaderProgram,
          "s_blendTexture"
        );
        if (shaderTypeIndex == 1 /* ShaderType_Masked */ || shaderTypeIndex == 2 /* ShaderType_MaskedInverted */) {
          this._shaderSets[shaderSetIndex].samplerTexture1Location = this.gl.getUniformLocation(
            this._shaderSets[shaderSetIndex].shaderProgram,
            "s_texture1"
          );
          this._shaderSets[shaderSetIndex].uniformClipMatrixLocation = this.gl.getUniformLocation(
            this._shaderSets[shaderSetIndex].shaderProgram,
            "u_clipMatrix"
          );
          this._shaderSets[shaderSetIndex].uniformChannelFlagLocation = this.gl.getUniformLocation(
            this._shaderSets[shaderSetIndex].shaderProgram,
            "u_channelFlag"
          );
          this._shaderSets[shaderSetIndex].uniformInvertMaskFlagLocation = this.gl.getUniformLocation(
            this._shaderSets[shaderSetIndex].shaderProgram,
            "u_invertClippingMask"
          );
        }
      }
    }
    /**
     * シェーダプログラムをロードしてアドレスを返す
     *
     * @param vertexShaderSource    頂点シェーダのソース
     * @param fragmentShaderSource  フラグメントシェーダのソース
     *
     * @return シェーダプログラムのアドレス
     */
    loadShaderProgram(vertexShaderSource, fragmentShaderSource) {
      let shaderProgram = this.gl.createProgram();
      let vertShader = this.compileShaderSource(
        this.gl.VERTEX_SHADER,
        vertexShaderSource
      );
      if (!vertShader) {
        CubismLogError("Vertex shader compile error!");
        return 0;
      }
      let fragShader = this.compileShaderSource(
        this.gl.FRAGMENT_SHADER,
        fragmentShaderSource
      );
      if (!fragShader) {
        CubismLogError("Fragment shader compile error!");
        return 0;
      }
      this.gl.attachShader(shaderProgram, vertShader);
      this.gl.attachShader(shaderProgram, fragShader);
      this.gl.linkProgram(shaderProgram);
      const linkStatus = this.gl.getProgramParameter(
        shaderProgram,
        this.gl.LINK_STATUS
      );
      if (!linkStatus) {
        const linkLog = this.gl.getProgramInfoLog(shaderProgram);
        console.error("Cubism shader link failed.", linkLog);
        CubismLogError("Failed to link program: {0}", shaderProgram);
        this.gl.deleteShader(vertShader);
        vertShader = 0;
        this.gl.deleteShader(fragShader);
        fragShader = 0;
        if (shaderProgram) {
          this.gl.deleteProgram(shaderProgram);
          shaderProgram = 0;
        }
        return 0;
      }
      this.gl.deleteShader(vertShader);
      this.gl.deleteShader(fragShader);
      return shaderProgram;
    }
    /**
     * シェーダープログラムをコンパイルする
     *
     * @param shaderType シェーダタイプ(Vertex/Fragment)
     * @param shaderSource シェーダソースコード
     *
     * @return コンパイルされたシェーダープログラム
     */
    compileShaderSource(shaderType, shaderSource) {
      const source = shaderSource;
      const shader = this.gl.createShader(shaderType);
      this.gl.shaderSource(shader, source);
      this.gl.compileShader(shader);
      if (!shader) {
        const log = this.gl.getShaderInfoLog(shader);
        console.error("Cubism shader compile failed.", log);
        CubismLogError("Shader compile log: {0} ", log);
      }
      const status = this.gl.getShaderParameter(
        shader,
        this.gl.COMPILE_STATUS
      );
      if (!status) {
        const log = this.gl.getShaderInfoLog(shader);
        console.error("Cubism shader compile failed.", log);
        CubismLogError("Shader compile log: {0} ", log);
        this.gl.deleteShader(shader);
        return null;
      }
      return shader;
    }
    /**
     * WebGLレンダリングコンテキストを設定する
     *
     * @param gl WebGLレンダリングコンテキスト
     */
    setGl(gl) {
      this.gl = gl;
    }
    /**
     * ブレンドモード用のシェーダーパスを設定する
     *
     * @param shaderPath シェーダーパス
     */
    setShaderPath(shaderPath) {
      this._shaderPath = shaderPath;
    }
    /**
     * シェーダーパスを取得する
     *
     * @return シェーダーパス
     */
    getShaderPath() {
      return this._shaderPath;
    }
    // シェーダーパス
  };
  var CubismShaderManager_WebGL = class _CubismShaderManager_WebGL {
    /**
     * インスタンスを取得する（シングルトン）
     *
     * @return インスタンス
     */
    static getInstance() {
      if (s_instance == null) {
        s_instance = new _CubismShaderManager_WebGL();
      }
      return s_instance;
    }
    /**
     * インスタンスを開放する（シングルトン）
     */
    static deleteInstance() {
      if (s_instance) {
        s_instance.release();
        s_instance = null;
      }
    }
    /**
     * Privateなコンストラクタ
     */
    constructor() {
      this._shaderMap = /* @__PURE__ */ new Map();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      for (const item of this._shaderMap) {
        item[1].release();
      }
      this._shaderMap.clear();
    }
    /**
     * GLContextをキーにShaderを取得する
     *
     * @param gl glコンテキスト
     *
     * @return shaderを返す
     */
    getShader(gl) {
      return this._shaderMap.get(gl);
    }
    /**
     * GLContextを登録する
     *
     * @param gl glコンテキスト
     */
    setGlContext(gl) {
      if (!this._shaderMap.has(gl)) {
        const instance = new CubismShader_WebGL();
        instance.setGl(gl);
        this._shaderMap.set(gl, instance);
      }
    }
  };
  var CubismShaderSet = class {
    // シェーダープログラムに渡す変数のアドレス（InvertMask）
  };
  var ShaderNames = /* @__PURE__ */ ((ShaderNames2) => {
    ShaderNames2[ShaderNames2["ShaderNames_SetupMask"] = 0] = "ShaderNames_SetupMask";
    ShaderNames2[ShaderNames2["ShaderNames_NormalPremultipliedAlpha"] = 1] = "ShaderNames_NormalPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_NormalMaskedPremultipliedAlpha"] = 2] = "ShaderNames_NormalMaskedPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_NomralMaskedInvertedPremultipliedAlpha"] = 3] = "ShaderNames_NomralMaskedInvertedPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_AddPremultipliedAlpha"] = 4] = "ShaderNames_AddPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_AddMaskedPremultipliedAlpha"] = 5] = "ShaderNames_AddMaskedPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_AddMaskedPremultipliedAlphaInverted"] = 6] = "ShaderNames_AddMaskedPremultipliedAlphaInverted";
    ShaderNames2[ShaderNames2["ShaderNames_MultPremultipliedAlpha"] = 7] = "ShaderNames_MultPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_MultMaskedPremultipliedAlpha"] = 8] = "ShaderNames_MultMaskedPremultipliedAlpha";
    ShaderNames2[ShaderNames2["ShaderNames_MultMaskedPremultipliedAlphaInverted"] = 9] = "ShaderNames_MultMaskedPremultipliedAlphaInverted";
    ShaderNames2[ShaderNames2["ShaderNames_ShaderCount"] = 10] = "ShaderNames_ShaderCount";
    return ShaderNames2;
  })(ShaderNames || {});
  var Live2DCubismFramework34;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismShaderSet = CubismShaderSet;
    Live2DCubismFramework51.CubismShader_WebGL = CubismShader_WebGL;
    Live2DCubismFramework51.CubismShaderManager_WebGL = CubismShaderManager_WebGL;
    Live2DCubismFramework51.ShaderNames = ShaderNames;
  })(Live2DCubismFramework34 || (Live2DCubismFramework34 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismoffscreenrendertarget_webgl.ts
  var CubismOffscreenRenderTarget_WebGL = class extends CubismRenderTarget_WebGL {
    /**
     * リソースコンテナマネージャを初期化する。
     *
     * @param displayBufferWidth レンダーターゲットの幅
     * @param displayBufferHeight レンダーターゲットの高さ
     */
    initializeOffscreenManager(gl, displayBufferWidth, displayBufferHeight) {
      this._gl = gl;
      this._webGLOffscreenManager = CubismWebGLOffscreenManager.getInstance();
      if (this._webGLOffscreenManager.getContainerSize(gl) === 0) {
        this._webGLOffscreenManager.initialize(
          gl,
          displayBufferWidth,
          displayBufferHeight
        );
      }
    }
    /**
     * オフスクリーン描画用レンダーターゲットをセットする。
     *
     * @param gl WebGLRenderingContextまたはWebGL2RenderingContext
     *          NOTE: Cubism 5.3以降のモデルが使用される場合はWebGL2RenderingContextを使用すること。
     * @param displayBufferWidth レンダーターゲットの幅
     * @param displayBufferHeight レンダーターゲットの高さ
     * @param previousFramebuffer 前のフレームバッファ
     */
    setOffscreenRenderTarget(gl, displayBufferWidth, displayBufferHeight, previousFramebuffer) {
      if (this._webGLOffscreenManager == null) {
        this.initializeOffscreenManager(
          gl,
          displayBufferWidth,
          displayBufferHeight
        );
      }
      const offscreenRenderTargetContainer = this._webGLOffscreenManager.getOffscreenRenderTargetContainers(
        gl,
        displayBufferWidth,
        displayBufferHeight,
        previousFramebuffer
      );
      if (offscreenRenderTargetContainer == null) {
        CubismLogError("Failed to acquire offscreen render texture container.");
        return;
      }
      this._colorBuffer = offscreenRenderTargetContainer.getColorBuffer();
      this._renderTexture = offscreenRenderTargetContainer.getRenderTexture();
      this._bufferWidth = displayBufferWidth;
      this._bufferHeight = displayBufferHeight;
      this._gl = gl;
      if (this._renderTexture == null) {
        this._renderTexture = previousFramebuffer;
        CubismLogError("Failed to create offscreen render texture.");
      }
      return;
    }
    /**
     * リソースコンテナの使用状態を取得
     *
     * @return 使用中はtrue、未使用の場合はfalse
     */
    getUsingRenderTextureState() {
      if (this._webGLOffscreenManager == null || this._gl == null) {
        return true;
      }
      return this._webGLOffscreenManager.getUsingRenderTextureState(
        this._gl,
        this._renderTexture
      );
    }
    /**
     * リソースコンテナの使用を開始する。
     */
    startUsingRenderTexture() {
      if (this._webGLOffscreenManager == null || this._gl == null) {
        return;
      }
      this._webGLOffscreenManager.startUsingRenderTexture(
        this._gl,
        this._renderTexture
      );
    }
    /**
     * リソースコンテナの使用を終了する。
     */
    stopUsingRenderTexture() {
      if (this._webGLOffscreenManager == null || this._gl == null) {
        return;
      }
      this._webGLOffscreenManager.stopUsingRenderTexture(
        this._gl,
        this._renderTexture
      );
    }
    /**
     * オフスクリーンのインデックスを設定する。
     *
     * @param offscreenIndex オフスクリーンのインデックス
     */
    setOffscreenIndex(offscreenIndex) {
      this._offscreenIndex = offscreenIndex;
    }
    /**
     * オフスクリーンのインデックスを取得する。
     *
     * @return オフスクリーンのインデックス
     */
    getOffscreenIndex() {
      return this._offscreenIndex;
    }
    /**
     * 以前のオフスクリーン描画用レンダーターゲットを設定する。
     *
     * @param oldOffscreen 以前のオフスクリーン描画用レンダーターゲット
     */
    setOldOffscreen(oldOffscreen) {
      this._oldOffscreen = oldOffscreen;
    }
    /**
     * 以前のオフスクリーン描画用レンダーターゲットを取得する。
     *
     * @return 以前のオフスクリーン描画用レンダーターゲット
     */
    getOldOffscreen() {
      return this._oldOffscreen;
    }
    /**
     * 親のオフスクリーン描画用レンダーターゲットを設定する。
     *
     * @param parentOffscreenRenderTarget 親のオフスクリーン描画用レンダーターゲット
     */
    setParentPartOffscreen(parentOffscreenRenderTarget) {
      this._parentOffscreenRenderTarget = parentOffscreenRenderTarget;
    }
    /**
     * 親のオフスクリーン描画用レンダーターゲットを取得する。
     *
     * @return 親のオフスクリーン描画用レンダーターゲット
     */
    getParentPartOffscreen() {
      return this._parentOffscreenRenderTarget;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._offscreenIndex = -1;
      this._parentOffscreenRenderTarget = null;
      this._oldOffscreen = null;
      this._webGLOffscreenManager = null;
    }
    release() {
      if (this._webGLOffscreenManager != null && this._gl != null && this._renderTexture != null) {
        this._webGLOffscreenManager.stopUsingRenderTexture(
          this._gl,
          this._renderTexture
        );
      }
      if (this._colorBuffer && this._gl) {
        this._gl.deleteTexture(this._colorBuffer);
        this._colorBuffer = null;
      }
      if (this._renderTexture && this._gl) {
        this._gl.deleteFramebuffer(this._renderTexture);
        this._renderTexture = null;
      }
      if (this._webGLOffscreenManager != null) {
        this._webGLOffscreenManager = null;
      }
      this._oldOffscreen = null;
      this._parentOffscreenRenderTarget = null;
    }
    // WebGLコンテキスト
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/rendering/cubismrenderer_webgl.ts
  var s_invalidValue = -1;
  var s_renderTargetIndexArray = new Uint16Array([
    0,
    1,
    2,
    2,
    1,
    3
  ]);
  var CubismClippingManager_WebGL = class extends CubismClippingManager {
    /**
     * WebGLレンダリングコンテキストを設定する
     *
     * @param gl WebGLレンダリングコンテキスト
     */
    setGL(gl) {
      this.gl = gl;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      super(CubismClippingContext_WebGL);
    }
    /**
     * クリッピングコンテキストを作成する。モデル描画時に実行する。
     *
     * @param model モデルのインスタンス
     * @param renderer レンダラのインスタンス
     * @param lastFbo フレームバッファ
     * @param lastViewport ビューポート
     * @param drawObjectType 描画オブジェクトのタイプ
     */
    setupClippingContext(model, renderer, lastFbo, lastViewport, drawObjectType) {
      let usingClipCount = 0;
      for (let clipIndex = 0; clipIndex < this._clippingContextListForMask.length; clipIndex++) {
        const cc = this._clippingContextListForMask[clipIndex];
        switch (drawObjectType) {
          case 0 /* DrawableObjectType_Drawable */:
          default:
            this.calcClippedDrawableTotalBounds(model, cc);
            break;
          case 1 /* DrawableObjectType_Offscreen */:
            this.calcClippedOffscreenTotalBounds(model, cc);
            break;
        }
        if (cc._isUsing) {
          usingClipCount++;
        }
      }
      if (usingClipCount <= 0) {
        return;
      }
      this.gl.viewport(
        0,
        0,
        this._clippingMaskBufferSize,
        this._clippingMaskBufferSize
      );
      switch (drawObjectType) {
        case 0 /* DrawableObjectType_Drawable */:
        default:
          this._currentMaskBuffer = renderer.getDrawableMaskBuffer(0);
          break;
        case 1 /* DrawableObjectType_Offscreen */:
          this._currentMaskBuffer = renderer.getOffscreenMaskBuffer(0);
          break;
      }
      this._currentMaskBuffer.beginDraw(lastFbo);
      renderer.preDraw();
      this.setupLayoutBounds(usingClipCount);
      if (this._clearedMaskBufferFlags.length != this._renderTextureCount) {
        this._clearedMaskBufferFlags.length = 0;
        this._clearedMaskBufferFlags = new Array(
          this._renderTextureCount
        );
        for (let i = 0; i < this._clearedMaskBufferFlags.length; i++) {
          this._clearedMaskBufferFlags[i] = false;
        }
      }
      for (let index = 0; index < this._clearedMaskBufferFlags.length; index++) {
        this._clearedMaskBufferFlags[index] = false;
      }
      for (let clipIndex = 0; clipIndex < this._clippingContextListForMask.length; clipIndex++) {
        const clipContext = this._clippingContextListForMask[clipIndex];
        const allClipedDrawRect = clipContext._allClippedDrawRect;
        const layoutBoundsOnTex01 = clipContext._layoutBounds;
        const margin = 0.05;
        let scaleX = 0;
        let scaleY = 0;
        let maskBuffer;
        switch (drawObjectType) {
          case 0 /* DrawableObjectType_Drawable */:
          default:
            maskBuffer = renderer.getDrawableMaskBuffer(clipContext._bufferIndex);
            break;
          case 1 /* DrawableObjectType_Offscreen */:
            maskBuffer = renderer.getOffscreenMaskBuffer(
              clipContext._bufferIndex
            );
            break;
        }
        if (this._currentMaskBuffer != maskBuffer) {
          this._currentMaskBuffer.endDraw();
          this._currentMaskBuffer = maskBuffer;
          this._currentMaskBuffer.beginDraw(lastFbo);
          renderer.preDraw();
        }
        this._tmpBoundsOnModel.setRect(allClipedDrawRect);
        this._tmpBoundsOnModel.expand(
          allClipedDrawRect.width * margin,
          allClipedDrawRect.height * margin
        );
        scaleX = layoutBoundsOnTex01.width / this._tmpBoundsOnModel.width;
        scaleY = layoutBoundsOnTex01.height / this._tmpBoundsOnModel.height;
        this.createMatrixForMask(false, layoutBoundsOnTex01, scaleX, scaleY);
        clipContext._matrixForMask.setMatrix(this._tmpMatrixForMask.getArray());
        clipContext._matrixForDraw.setMatrix(this._tmpMatrixForDraw.getArray());
        if (drawObjectType == 1 /* DrawableObjectType_Offscreen */) {
          const invertMvp = renderer.getMvpMatrix().getInvert();
          clipContext._matrixForDraw.multiplyByMatrix(invertMvp);
        }
        const clipDrawCount = clipContext._clippingIdCount;
        for (let i = 0; i < clipDrawCount; i++) {
          const clipDrawIndex = clipContext._clippingIdList[i];
          if (!model.getDrawableDynamicFlagVertexPositionsDidChange(clipDrawIndex)) {
            continue;
          }
          renderer.setIsCulling(model.getDrawableCulling(clipDrawIndex) != false);
          if (!this._clearedMaskBufferFlags[clipContext._bufferIndex]) {
            this.gl.clearColor(1, 1, 1, 1);
            this.gl.clear(this.gl.COLOR_BUFFER_BIT);
            this._clearedMaskBufferFlags[clipContext._bufferIndex] = true;
          }
          renderer.setClippingContextBufferForMask(clipContext);
          renderer.drawMeshWebGL(model, clipDrawIndex);
        }
      }
      this._currentMaskBuffer.endDraw();
      renderer.setClippingContextBufferForMask(null);
      this.gl.viewport(
        lastViewport[0],
        lastViewport[1],
        lastViewport[2],
        lastViewport[3]
      );
    }
    /**
     * マスクの合計数をカウント
     *
     * @return マスクの合計数を返す
     */
    getClippingMaskCount() {
      return this._clippingContextListForMask.length;
    }
    // WebGLレンダリングコンテキスト
  };
  var CubismClippingContext_WebGL = class extends CubismClippingContext {
    /**
     * 引数付きコンストラクタ
     *
     * @param manager マスクを管理しているマネージャのインスタンス
     * @param clippingDrawableIndices クリップしているDrawableのインデックスリスト
     * @param clipCount クリップしているDrawableの個数
     */
    constructor(manager, clippingDrawableIndices, clipCount) {
      super(clippingDrawableIndices, clipCount);
      this._owner = manager;
    }
    /**
     * このマスクを管理するマネージャのインスタンスを取得する
     *
     * @return クリッピングマネージャのインスタンス
     */
    getClippingManager() {
      return this._owner;
    }
    /**
     * WebGLレンダリングコンテキストを設定する
     *
     * @param gl WebGLレンダリングコンテキスト
     */
    setGl(gl) {
      this._owner.setGL(gl);
    }
    // このマスクを管理しているマネージャのインスタンス
  };
  var CubismRendererProfile_WebGL = class {
    /**
     * WebGLの有効・無効をセットする
     *
     * @param index 有効・無効にする機能
     * @param enabled trueなら有効にする
     */
    setGlEnable(index, enabled) {
      if (enabled) this.gl.enable(index);
      else this.gl.disable(index);
    }
    /**
     * WebGLのVertex Attribute Array機能の有効・無効をセットする
     *
     * @param   index   有効・無効にする機能
     * @param   enabled trueなら有効にする
     */
    setGlEnableVertexAttribArray(index, enabled) {
      if (enabled) this.gl.enableVertexAttribArray(index);
      else this.gl.disableVertexAttribArray(index);
    }
    /**
     * WebGLのステートを保持する
     */
    save() {
      if (this.gl == null) {
        CubismLogError(
          "'gl' is null. WebGLRenderingContext is required.\nPlease call 'CubimRenderer_WebGL.startUp' function."
        );
        return;
      }
      this._lastArrayBufferBinding = this.gl.getParameter(
        this.gl.ARRAY_BUFFER_BINDING
      );
      this._lastElementArrayBufferBinding = this.gl.getParameter(
        this.gl.ELEMENT_ARRAY_BUFFER_BINDING
      );
      this._lastProgram = this.gl.getParameter(this.gl.CURRENT_PROGRAM);
      this._lastActiveTexture = this.gl.getParameter(this.gl.ACTIVE_TEXTURE);
      this.gl.activeTexture(this.gl.TEXTURE1);
      this._lastTexture1Binding2D = this.gl.getParameter(
        this.gl.TEXTURE_BINDING_2D
      );
      this.gl.activeTexture(this.gl.TEXTURE0);
      this._lastTexture0Binding2D = this.gl.getParameter(
        this.gl.TEXTURE_BINDING_2D
      );
      this._lastVertexAttribArrayEnabled[0] = this.gl.getVertexAttrib(
        0,
        this.gl.VERTEX_ATTRIB_ARRAY_ENABLED
      );
      this._lastVertexAttribArrayEnabled[1] = this.gl.getVertexAttrib(
        1,
        this.gl.VERTEX_ATTRIB_ARRAY_ENABLED
      );
      this._lastVertexAttribArrayEnabled[2] = this.gl.getVertexAttrib(
        2,
        this.gl.VERTEX_ATTRIB_ARRAY_ENABLED
      );
      this._lastVertexAttribArrayEnabled[3] = this.gl.getVertexAttrib(
        3,
        this.gl.VERTEX_ATTRIB_ARRAY_ENABLED
      );
      this._lastScissorTest = this.gl.isEnabled(this.gl.SCISSOR_TEST);
      this._lastStencilTest = this.gl.isEnabled(this.gl.STENCIL_TEST);
      this._lastDepthTest = this.gl.isEnabled(this.gl.DEPTH_TEST);
      this._lastCullFace = this.gl.isEnabled(this.gl.CULL_FACE);
      this._lastBlend = this.gl.isEnabled(this.gl.BLEND);
      this._lastFrontFace = this.gl.getParameter(this.gl.FRONT_FACE);
      this._lastColorMask = this.gl.getParameter(this.gl.COLOR_WRITEMASK);
      this._lastBlending[0] = this.gl.getParameter(this.gl.BLEND_SRC_RGB);
      this._lastBlending[1] = this.gl.getParameter(this.gl.BLEND_DST_RGB);
      this._lastBlending[2] = this.gl.getParameter(this.gl.BLEND_SRC_ALPHA);
      this._lastBlending[3] = this.gl.getParameter(this.gl.BLEND_DST_ALPHA);
    }
    /**
     * 保持したWebGLのステートを復帰させる
     */
    restore() {
      if (this.gl == null) {
        CubismLogError(
          "'gl' is null. WebGLRenderingContext is required.\nPlease call 'CubimRenderer_WebGL.startUp' function."
        );
        return;
      }
      this.gl.useProgram(this._lastProgram);
      this.setGlEnableVertexAttribArray(0, this._lastVertexAttribArrayEnabled[0]);
      this.setGlEnableVertexAttribArray(1, this._lastVertexAttribArrayEnabled[1]);
      this.setGlEnableVertexAttribArray(2, this._lastVertexAttribArrayEnabled[2]);
      this.setGlEnableVertexAttribArray(3, this._lastVertexAttribArrayEnabled[3]);
      this.setGlEnable(this.gl.SCISSOR_TEST, this._lastScissorTest);
      this.setGlEnable(this.gl.STENCIL_TEST, this._lastStencilTest);
      this.setGlEnable(this.gl.DEPTH_TEST, this._lastDepthTest);
      this.setGlEnable(this.gl.CULL_FACE, this._lastCullFace);
      this.setGlEnable(this.gl.BLEND, this._lastBlend);
      this.gl.frontFace(this._lastFrontFace);
      this.gl.colorMask(
        this._lastColorMask[0],
        this._lastColorMask[1],
        this._lastColorMask[2],
        this._lastColorMask[3]
      );
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this._lastArrayBufferBinding);
      this.gl.bindBuffer(
        this.gl.ELEMENT_ARRAY_BUFFER,
        this._lastElementArrayBufferBinding
      );
      this.gl.activeTexture(this.gl.TEXTURE1);
      this.gl.bindTexture(this.gl.TEXTURE_2D, this._lastTexture1Binding2D);
      this.gl.activeTexture(this.gl.TEXTURE0);
      this.gl.bindTexture(this.gl.TEXTURE_2D, this._lastTexture0Binding2D);
      this.gl.activeTexture(this._lastActiveTexture);
      this.gl.blendFuncSeparate(
        this._lastBlending[0],
        this._lastBlending[1],
        this._lastBlending[2],
        this._lastBlending[3]
      );
    }
    /**
     * WebGLレンダリングコンテキストを設定する
     *
     * @param gl WebGLレンダリングコンテキスト
     */
    setGl(gl) {
      this.gl = gl;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._lastVertexAttribArrayEnabled = new Array(4);
      this._lastColorMask = new Array(4);
      this._lastBlending = new Array(4);
    }
  };
  var CubismRenderer_WebGL = class extends CubismRenderer {
    /**
     * レンダラの初期化処理を実行する
     * 引数に渡したモデルからレンダラの初期化処理に必要な情報を取り出すことができる
     * NOTE: WebGLコンテキストが初期化されていない可能性があるため、ここではWebGLコンテキストを使う初期化は行わない。
     *
     * @param model モデルのインスタンス
     * @param maskBufferCount バッファの生成数
     */
    initialize(model, maskBufferCount = 1) {
      if (model.isUsingMasking()) {
        this._drawableClippingManager = new CubismClippingManager_WebGL();
        this._drawableClippingManager.initializeForDrawable(
          model,
          maskBufferCount
        );
      }
      if (model.isUsingMaskingForOffscreen()) {
        this._offscreenClippingManager = new CubismClippingManager_WebGL();
        this._offscreenClippingManager.initializeForOffscreen(
          model,
          maskBufferCount
        );
      }
      updateSize(
        this._sortedObjectsIndexList,
        model.getDrawableCount() + (model.getOffscreenCount ? model.getOffscreenCount() : 0),
        0,
        true
      );
      updateSize(
        this._sortedObjectsTypeList,
        model.getDrawableCount() + (model.getOffscreenCount ? model.getOffscreenCount() : 0),
        0,
        true
      );
      super.initialize(model);
    }
    /**
     * オフスクリーンの親を探して設定する
     *
     * @param model モデルのインスタンス
     * @param offscreenCount オフスクリーンの数
     */
    setupParentOffscreens(model, offscreenCount) {
      let parentOffscreen;
      for (let offscreenIndex = 0; offscreenIndex < offscreenCount; ++offscreenIndex) {
        parentOffscreen = null;
        const ownerIndex = model.getOffscreenOwnerIndices()[offscreenIndex];
        let parentIndex = model.getPartParentPartIndices()[ownerIndex];
        while (parentIndex != NoParentIndex) {
          for (let i = 0; i < offscreenCount; ++i) {
            const ownerIndex2 = model.getOffscreenOwnerIndices()[this._offscreenList[i].getOffscreenIndex()];
            if (ownerIndex2 != parentIndex) {
              continue;
            }
            parentOffscreen = this._offscreenList[i];
            break;
          }
          if (parentOffscreen != null) {
            break;
          }
          parentIndex = model.getPartParentPartIndices()[parentIndex];
        }
        this._offscreenList[offscreenIndex].setParentPartOffscreen(
          parentOffscreen
        );
      }
    }
    /**
     * WebGLテクスチャのバインド処理
     * CubismRendererにテクスチャを設定し、CubismRenderer内でその画像を参照するためのIndex値を戻り値とする
     *
     * @param modelTextureNo セットするモデルテクスチャの番号
     * @param glTextureNo WebGLテクスチャの番号
     */
    bindTexture(modelTextureNo, glTexture) {
      this._textures.set(modelTextureNo, glTexture);
    }
    /**
     * WebGLにバインドされたテクスチャのリストを取得する
     *
     * @return テクスチャのリスト
     */
    getBindedTextures() {
      return this._textures;
    }
    /**
     * クリッピングマスクバッファのサイズを設定する
     * マスク用のFrameBufferを破棄、再作成する為処理コストは高い
     *
     * @param size クリッピングマスクバッファのサイズ
     */
    setClippingMaskBufferSize(size) {
      if (!this._model.isUsingMasking()) {
        return;
      }
      const renderTextureCount = this._drawableClippingManager.getRenderTextureCount();
      this._drawableClippingManager.release();
      this._drawableClippingManager = void 0;
      this._drawableClippingManager = null;
      this._drawableClippingManager = new CubismClippingManager_WebGL();
      this._drawableClippingManager.setClippingMaskBufferSize(size);
      this._drawableClippingManager.initializeForDrawable(
        this.getModel(),
        renderTextureCount
        // インスタンス破棄前に保存したレンダーテクスチャの数
      );
    }
    /**
     * クリッピングマスクバッファのサイズを取得する
     *
     * @return クリッピングマスクバッファのサイズ
     */
    getClippingMaskBufferSize() {
      return this._model.isUsingMasking() ? this._drawableClippingManager.getClippingMaskBufferSize() : s_invalidValue;
    }
    /**
     * ブレンドモード用のフレームバッファを取得する
     *
     * @return ブレンドモード用のフレームバッファ
     */
    getModelRenderTarget(index) {
      return this._modelRenderTargets[index];
    }
    /**
     * レンダーテクスチャの枚数を取得する
     * @return レンダーテクスチャの枚数
     */
    getRenderTextureCount() {
      return this._model.isUsingMasking() ? this._drawableClippingManager.getRenderTextureCount() : s_invalidValue;
    }
    /**
     * コンストラクタ
     */
    constructor(width, height) {
      super(width, height);
      this._clippingContextBufferForMask = null;
      this._clippingContextBufferForDraw = null;
      this._rendererProfile = new CubismRendererProfile_WebGL();
      this._textures = /* @__PURE__ */ new Map();
      this._sortedObjectsIndexList = new Array();
      this._sortedObjectsTypeList = new Array();
      this._bufferData = {
        vertex: WebGLBuffer = null,
        uv: WebGLBuffer = null,
        index: WebGLBuffer = null
      };
      this._modelRenderTargets = new Array();
      this._drawableMasks = new Array();
      this._currentFbo = null;
      this._drawableClippingManager = null;
      this._offscreenClippingManager = null;
      this._offscreenMasks = new Array();
      this._offscreenList = new Array();
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      if (this._drawableClippingManager) {
        this._drawableClippingManager.release();
        this._drawableClippingManager = void 0;
        this._drawableClippingManager = null;
      }
      if (this.gl == null) {
        return;
      }
      this.gl.deleteBuffer(this._bufferData.vertex);
      this._bufferData.vertex = null;
      this.gl.deleteBuffer(this._bufferData.uv);
      this._bufferData.uv = null;
      this.gl.deleteBuffer(this._bufferData.index);
      this._bufferData.index = null;
      this._bufferData = null;
      this._textures = null;
      for (let i = 0; i < this._modelRenderTargets.length; i++) {
        if (this._modelRenderTargets[i] != null && this._modelRenderTargets[i].isValid()) {
          this._modelRenderTargets[i].destroyRenderTarget();
        }
      }
      this._modelRenderTargets.length = 0;
      this._modelRenderTargets = null;
      for (let i = 0; i < this._drawableMasks.length; i++) {
        if (this._drawableMasks[i] != null && this._drawableMasks[i].isValid()) {
          this._drawableMasks[i].destroyRenderTarget();
        }
      }
      this._drawableMasks.length = 0;
      this._drawableMasks = null;
      for (let i = 0; i < this._offscreenMasks.length; i++) {
        if (this._offscreenMasks[i] != null && this._offscreenMasks[i].isValid()) {
          this._offscreenMasks[i].destroyRenderTarget();
        }
      }
      this._offscreenMasks.length = 0;
      this._offscreenMasks = null;
      for (let i = 0; i < this._offscreenList.length; i++) {
        if (this._offscreenList[i] != null && this._offscreenList[i].isValid()) {
          this._offscreenList[i].destroyRenderTarget();
        }
      }
      this._offscreenList.length = 0;
      this._offscreenList = null;
      this._offscreenClippingManager = null;
      this._drawableClippingManager = null;
      this._clippingContextBufferForMask = null;
      this._clippingContextBufferForDraw = null;
      this._rendererProfile = null;
      this._sortedObjectsIndexList = null;
      this._sortedObjectsTypeList = null;
      this._currentFbo = null;
      this._model = null;
      this.gl = null;
    }
    /**
     * Shaderの読み込みを行う
     * @param shaderPath シェーダのパス
     */
    loadShaders(shaderPath = null) {
      if (this.gl == null) {
        CubismLogError(
          "'gl' is null. WebGLRenderingContext is required.\nPlease call 'CubimRenderer_WebGL.startUp' function."
        );
        return;
      }
      if (CubismShaderManager_WebGL.getInstance().getShader(this.gl)._shaderSets.length == 0 || !CubismShaderManager_WebGL.getInstance().getShader(this.gl)._isShaderLoaded) {
        const shader = CubismShaderManager_WebGL.getInstance().getShader(this.gl);
        if (shaderPath != null) {
          shader.setShaderPath(shaderPath);
        }
        shader.generateShaders();
      }
    }
    /**
     * モデルを描画する実際の処理
     * @param shaderPath シェーダのパス
     */
    doDrawModel(shaderPath = null) {
      this.loadShaders(shaderPath);
      this.beforeDrawModelRenderTarget();
      const lastFbo = this.gl.getParameter(
        this.gl.FRAMEBUFFER_BINDING
      );
      const lastViewport = this.gl.getParameter(this.gl.VIEWPORT);
      if (this._drawableClippingManager != null) {
        this.preDraw();
        for (let i = 0; i < this._drawableClippingManager.getRenderTextureCount(); ++i) {
          if (this._drawableMasks[i].getBufferWidth() != this._drawableClippingManager.getClippingMaskBufferSize() || this._drawableMasks[i].getBufferHeight() != this._drawableClippingManager.getClippingMaskBufferSize()) {
            this._drawableMasks[i].createRenderTarget(
              this.gl,
              this._drawableClippingManager.getClippingMaskBufferSize(),
              this._drawableClippingManager.getClippingMaskBufferSize(),
              lastFbo
            );
          }
        }
        if (this.isUsingHighPrecisionMask()) {
          this._drawableClippingManager.setupMatrixForHighPrecision(
            this.getModel(),
            false
          );
        } else {
          this._drawableClippingManager.setupClippingContext(
            this.getModel(),
            this,
            lastFbo,
            lastViewport,
            0 /* DrawableObjectType_Drawable */
          );
        }
      }
      if (this._offscreenClippingManager != null) {
        this.preDraw();
        for (let i = 0; i < this._offscreenClippingManager.getRenderTextureCount(); ++i) {
          if (this._offscreenMasks[i].getBufferWidth() != this._offscreenClippingManager.getClippingMaskBufferSize() || this._offscreenMasks[i].getBufferHeight() != this._offscreenClippingManager.getClippingMaskBufferSize()) {
            this._offscreenMasks[i].createRenderTarget(
              this.gl,
              this._offscreenClippingManager.getClippingMaskBufferSize(),
              this._offscreenClippingManager.getClippingMaskBufferSize(),
              lastFbo
            );
          }
        }
        if (this.isUsingHighPrecisionMask()) {
          this._offscreenClippingManager.setupMatrixForOffscreenHighPrecision(
            this.getModel(),
            false,
            this.getMvpMatrix()
          );
        } else {
          this._offscreenClippingManager.setupClippingContext(
            this.getModel(),
            this,
            lastFbo,
            lastViewport,
            1 /* DrawableObjectType_Offscreen */
          );
        }
      }
      this.preDraw();
      this.drawObjectLoop(lastFbo);
      this.afterDrawModelRenderTarget();
    }
    /**
     * 描画オブジェクトのループ処理を行う。
     *
     * @param lastFbo 前回のフレームバッファ
     */
    drawObjectLoop(lastFbo) {
      const model = this.getModel();
      const drawableCount = model.getDrawableCount();
      const offscreenCount = model.getOffscreenCount();
      const totalCount = drawableCount + offscreenCount;
      const renderOrder = model.getRenderOrders();
      this._currentOffscreen = null;
      this._currentFbo = lastFbo;
      this._modelRootFbo = lastFbo;
      for (let i = 0; i < totalCount; ++i) {
        const order = renderOrder[i];
        if (i < drawableCount) {
          this._sortedObjectsIndexList[order] = i;
          this._sortedObjectsTypeList[order] = 0 /* DrawableObjectType_Drawable */;
        } else if (i < totalCount) {
          this._sortedObjectsIndexList[order] = i - drawableCount;
          this._sortedObjectsTypeList[order] = 1 /* DrawableObjectType_Offscreen */;
        }
      }
      for (let i = 0; i < totalCount; ++i) {
        const objectIndex = this._sortedObjectsIndexList[i];
        const objectType = this._sortedObjectsTypeList[i];
        this.renderObject(objectIndex, objectType);
      }
      while (this._currentOffscreen != null) {
        this.submitDrawToParentOffscreen(
          this._currentOffscreen.getOffscreenIndex(),
          1 /* DrawableObjectType_Offscreen */
        );
      }
    }
    /**
     * 描画オブジェクトを描画する。
     *
     * @param objectIndex 描画対象のオブジェクトのインデックス
     * @param objectType 描画対象のオブジェクトのタイプ
     * @param lastFbo 前回のフレームバッファ
     * @param lastViewport 前回のビューポート
     */
    renderObject(objectIndex, objectType) {
      switch (objectType) {
        case 0 /* DrawableObjectType_Drawable */:
          this.drawDrawable(objectIndex, this._modelRootFbo);
          break;
        case 1 /* DrawableObjectType_Offscreen */:
          this.addOffscreen(objectIndex);
          break;
        default:
          CubismLogError("Unknown object type: " + objectType);
          break;
      }
    }
    /**
     * 描画オブジェクト（アートメッシュ）を描画する。
     *
     * @param model 描画対象のモデル
     * @param index 描画対象のメッシュのインデックス
     */
    drawDrawable(drawableIndex, rootFbo) {
      if (!this.getModel().getDrawableDynamicFlagIsVisible(drawableIndex)) {
        return;
      }
      this.submitDrawToParentOffscreen(
        drawableIndex,
        0 /* DrawableObjectType_Drawable */
      );
      const clipContext = this._drawableClippingManager != null ? this._drawableClippingManager.getClippingContextListForDraw()[drawableIndex] : null;
      if (clipContext != null && this.isUsingHighPrecisionMask()) {
        if (clipContext._isUsing) {
          this.gl.viewport(
            0,
            0,
            this._drawableClippingManager.getClippingMaskBufferSize(),
            this._drawableClippingManager.getClippingMaskBufferSize()
          );
          this.preDraw();
          this.getDrawableMaskBuffer(clipContext._bufferIndex).beginDraw(
            this._currentFbo
          );
          this.gl.clearColor(1, 1, 1, 1);
          this.gl.clear(this.gl.COLOR_BUFFER_BIT);
        }
        {
          const clipDrawCount = clipContext._clippingIdCount;
          for (let index = 0; index < clipDrawCount; index++) {
            const clipDrawIndex = clipContext._clippingIdList[index];
            if (!this._model.getDrawableDynamicFlagVertexPositionsDidChange(
              clipDrawIndex
            )) {
              continue;
            }
            this.setIsCulling(
              this._model.getDrawableCulling(clipDrawIndex) != false
            );
            this.setClippingContextBufferForMask(clipContext);
            this.drawMeshWebGL(this._model, clipDrawIndex);
          }
          this.getDrawableMaskBuffer(clipContext._bufferIndex).endDraw();
          this.setClippingContextBufferForMask(null);
          this.gl.viewport(
            0,
            0,
            this._modelRenderTargetWidth,
            this._modelRenderTargetHeight
          );
          this.preDraw();
        }
      }
      this.setClippingContextBufferForDrawable(clipContext);
      this.setIsCulling(this.getModel().getDrawableCulling(drawableIndex));
      this.drawMeshWebGL(this._model, drawableIndex);
    }
    /**
     * 描画オブジェクト（アートメッシュ）を描画する。
     *
     * @param model 描画対象のモデル
     * @param index 描画対象のメッシュのインデックス
     */
    drawMeshWebGL(model, index) {
      if (this.isCulling()) {
        this.gl.enable(this.gl.CULL_FACE);
      } else {
        this.gl.disable(this.gl.CULL_FACE);
      }
      this.gl.frontFace(this.gl.CCW);
      if (this.isGeneratingMask()) {
        CubismShaderManager_WebGL.getInstance().getShader(this.gl).setupShaderProgramForMask(this, model, index);
      } else {
        CubismShaderManager_WebGL.getInstance().getShader(this.gl).setupShaderProgramForDrawable(this, model, index);
      }
      if (!CubismShaderManager_WebGL.getInstance().getShader(this.gl)._isShaderLoaded) {
        return;
      }
      {
        const indexCount = model.getDrawableVertexIndexCount(index);
        this.gl.drawElements(
          this.gl.TRIANGLES,
          indexCount,
          this.gl.UNSIGNED_SHORT,
          0
        );
      }
      this.gl.useProgram(null);
      this.setClippingContextBufferForDrawable(null);
      this.setClippingContextBufferForMask(null);
    }
    /**
     * オフスクリーンを親のオフスクリーンにコピーする。
     *
     * @param objectIndex オブジェクトのインデックス
     * @param objectType  オブジェクトの種類
     */
    submitDrawToParentOffscreen(objectIndex, objectType) {
      if (this._currentOffscreen == null || objectIndex == s_invalidValue) {
        return;
      }
      const currentOwnerIndex = this.getModel().getOffscreenOwnerIndices()[this._currentOffscreen.getOffscreenIndex()];
      if (currentOwnerIndex == s_invalidValue) {
        return;
      }
      let targetParentIndex = NoParentIndex;
      switch (objectType) {
        case 0 /* DrawableObjectType_Drawable */:
          targetParentIndex = this.getModel().getDrawableParentPartIndex(objectIndex);
          break;
        case 1 /* DrawableObjectType_Offscreen */:
          targetParentIndex = this.getModel().getPartParentPartIndices()[this.getModel().getOffscreenOwnerIndices()[objectIndex]];
          break;
        default:
          return;
      }
      while (targetParentIndex != NoParentIndex) {
        if (targetParentIndex == currentOwnerIndex) {
          return;
        }
        targetParentIndex = this.getModel().getPartParentPartIndices()[targetParentIndex];
      }
      this.drawOffscreen(this._currentOffscreen);
      this.submitDrawToParentOffscreen(objectIndex, objectType);
    }
    /**
     * 描画オブジェクト（オフスクリーン）を追加する。
     *
     * @param offscreenIndex オフスクリーンのインデックス
     */
    addOffscreen(offscreenIndex) {
      if (this._currentOffscreen != null && this._currentOffscreen.getOffscreenIndex() != offscreenIndex) {
        let isParent = false;
        const ownerIndex = this.getModel().getOffscreenOwnerIndices()[offscreenIndex];
        let parentIndex = this.getModel().getPartParentPartIndices()[ownerIndex];
        const currentOffscreenIndex = this._currentOffscreen.getOffscreenIndex();
        const currentOffscreenOwnerIndex = this.getModel().getOffscreenOwnerIndices()[currentOffscreenIndex];
        while (parentIndex != NoParentIndex) {
          if (parentIndex == currentOffscreenOwnerIndex) {
            isParent = true;
            break;
          }
          parentIndex = this.getModel().getPartParentPartIndices()[parentIndex];
        }
        if (!isParent) {
          this.submitDrawToParentOffscreen(
            offscreenIndex,
            1 /* DrawableObjectType_Offscreen */
          );
        }
      }
      const offscreen = this._offscreenList[offscreenIndex];
      if (offscreen.getRenderTexture() == null || offscreen.getBufferWidth() != this._modelRenderTargetWidth || offscreen.getBufferHeight() != this._modelRenderTargetHeight || offscreen.getUsingRenderTextureState()) {
        offscreen.setOffscreenRenderTarget(
          this.gl,
          this._modelRenderTargetWidth,
          this._modelRenderTargetHeight,
          this._currentFbo
        );
      } else {
        offscreen.startUsingRenderTexture();
      }
      const oldOffscreen = offscreen.getParentPartOffscreen();
      offscreen.setOldOffscreen(oldOffscreen);
      let oldFBO = null;
      if (oldOffscreen != null) {
        oldFBO = oldOffscreen.getRenderTexture();
      }
      if (oldFBO == null) {
        oldFBO = this._modelRootFbo;
      }
      offscreen.beginDraw(oldFBO);
      this.gl.viewport(
        0,
        0,
        this._modelRenderTargetWidth,
        this._modelRenderTargetHeight
      );
      offscreen.clear(0, 0, 0, 0);
      this._currentOffscreen = offscreen;
      this._currentFbo = offscreen.getRenderTexture();
    }
    /**
     * オフスクリーン描画を行う。
     *
     * @param offscreen オフスクリーンレンダリングターゲット
     */
    drawOffscreen(offscreen) {
      const offscreenIndex = offscreen.getOffscreenIndex();
      const clipContext = this._offscreenClippingManager != null ? this._offscreenClippingManager.getClippingContextListForOffscreen()[offscreenIndex] : null;
      if (clipContext != null && this.isUsingHighPrecisionMask()) {
        if (clipContext._isUsing) {
          this.gl.viewport(
            0,
            0,
            this._offscreenClippingManager.getClippingMaskBufferSize(),
            this._offscreenClippingManager.getClippingMaskBufferSize()
          );
          this.preDraw();
          this.getOffscreenMaskBuffer(clipContext._bufferIndex).beginDraw(
            this._currentFbo
          );
          this.gl.clearColor(1, 1, 1, 1);
          this.gl.clear(this.gl.COLOR_BUFFER_BIT);
        }
        {
          const clipDrawCount = clipContext._clippingIdCount;
          for (let index = 0; index < clipDrawCount; index++) {
            const clipDrawIndex = clipContext._clippingIdList[index];
            if (!this.getModel().getDrawableDynamicFlagVertexPositionsDidChange(
              clipDrawIndex
            )) {
              continue;
            }
            this.setIsCulling(
              this.getModel().getDrawableCulling(clipDrawIndex) != false
            );
            this.setClippingContextBufferForMask(clipContext);
            this.drawMeshWebGL(this.getModel(), clipDrawIndex);
          }
        }
        {
          this.getOffscreenMaskBuffer(clipContext._bufferIndex).endDraw();
          this.setClippingContextBufferForMask(null);
          this.gl.viewport(
            0,
            0,
            this._modelRenderTargetWidth,
            this._modelRenderTargetHeight
          );
          this.preDraw();
        }
      }
      this.setClippingContextBufferForOffscreen(clipContext);
      this.setIsCulling(this._model.getOffscreenCulling(offscreenIndex) != false);
      this.drawOffscreenWebGL(this.getModel(), offscreen);
    }
    /**
     * オフスクリーン描画のWebGL実装
     *
     * @param model モデル
     * @param index オフスクリーンインデックス
     */
    drawOffscreenWebGL(model, offscreen) {
      if (this.isCulling()) {
        this.gl.enable(this.gl.CULL_FACE);
      } else {
        this.gl.disable(this.gl.CULL_FACE);
      }
      this.gl.frontFace(this.gl.CCW);
      CubismShaderManager_WebGL.getInstance().getShader(this.gl).setupShaderProgramForOffscreen(this, model, offscreen);
      offscreen.endDraw();
      this._currentOffscreen = this._currentOffscreen.getOldOffscreen();
      this._currentFbo = offscreen.getOldFBO();
      if (this._currentFbo == null) {
        this._currentOffscreen = this._modelRenderTargets[0];
        this._currentFbo = this._modelRenderTargets[0].getRenderTexture();
        this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, this._currentFbo);
      }
      {
        const indexBuffer = this.gl.createBuffer();
        this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
        this.gl.bufferData(
          this.gl.ELEMENT_ARRAY_BUFFER,
          s_renderTargetIndexArray,
          this.gl.STATIC_DRAW
        );
        this.gl.drawElements(
          this.gl.TRIANGLES,
          s_renderTargetIndexArray.length,
          this.gl.UNSIGNED_SHORT,
          0
        );
        this.gl.deleteBuffer(indexBuffer);
      }
      offscreen.stopUsingRenderTexture();
      this.gl.useProgram(null);
      this.setClippingContextBufferForMask(null);
      this.setClippingContextBufferForOffscreen(null);
    }
    /**
     * モデル描画直前のレンダラのステートを保持する
     */
    saveProfile() {
      this._rendererProfile.save();
    }
    /**
     * モデル描画直前のレンダラのステートを復帰させる
     */
    restoreProfile() {
      this._rendererProfile.restore();
    }
    /**
     * モデル描画直前のオフスクリーン設定を行う
     */
    beforeDrawModelRenderTarget() {
      if (this._modelRenderTargets.length == 0) {
        return;
      }
      for (let i = 0; i < this._modelRenderTargets.length; ++i) {
        if (this._modelRenderTargets[i].getBufferWidth() != this._modelRenderTargetWidth || this._modelRenderTargets[i].getBufferHeight() != this._modelRenderTargetHeight) {
          this._modelRenderTargets[i].createRenderTarget(
            this.gl,
            this._modelRenderTargetWidth,
            this._modelRenderTargetHeight,
            this._currentFbo
          );
        }
      }
      this._modelRenderTargets[0].beginDraw();
      this._modelRenderTargets[0].clear(0, 0, 0, 0);
    }
    /**
     * モデル描画後のオフスクリーン設定を行う
     */
    afterDrawModelRenderTarget() {
      if (this._modelRenderTargets.length == 0) {
        return;
      }
      this._modelRenderTargets[0].endDraw();
      CubismShaderManager_WebGL.getInstance().getShader(this.gl).setupShaderProgramForOffscreenRenderTarget(this);
      if (CubismShaderManager_WebGL.getInstance().getShader(this.gl)._isShaderLoaded) {
        const indexBuffer = this.gl.createBuffer();
        this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
        this.gl.bufferData(
          this.gl.ELEMENT_ARRAY_BUFFER,
          s_renderTargetIndexArray,
          this.gl.STATIC_DRAW
        );
        this.gl.drawElements(
          this.gl.TRIANGLES,
          s_renderTargetIndexArray.length,
          this.gl.UNSIGNED_SHORT,
          0
        );
        this.gl.deleteBuffer(indexBuffer);
      }
      this.gl.useProgram(null);
    }
    /**
     * オフスクリーンのクリッピングマスクのバッファを取得する
     *
     * @param index オフスクリーンのクリッピングマスクのバッファのインデックス
     *
     * @return オフスクリーンのクリッピングマスクのバッファへのポインタ
     */
    getOffscreenMaskBuffer(index) {
      return this._offscreenMasks[index];
    }
    /**
     * レンダラが保持する静的なリソースを解放する
     * WebGLの静的なシェーダープログラムを解放する
     */
    static doStaticRelease() {
      CubismShaderManager_WebGL.deleteInstance();
    }
    /**
     * レンダーステートを設定する
     *
     * @param fbo アプリケーション側で指定しているフレームバッファ
     * @param viewport ビューポート
     */
    setRenderState(fbo, viewport) {
      this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, fbo);
      this.gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
      if (this._modelRenderTargetWidth != viewport[2] || this._modelRenderTargetHeight != viewport[3]) {
        this._modelRenderTargetWidth = viewport[2];
        this._modelRenderTargetHeight = viewport[3];
      }
    }
    /**
     * 描画開始時の追加処理
     * モデルを描画する前にクリッピングマスクに必要な処理を実装している
     */
    preDraw() {
      this.gl.disable(this.gl.SCISSOR_TEST);
      this.gl.disable(this.gl.STENCIL_TEST);
      this.gl.disable(this.gl.DEPTH_TEST);
      this.gl.frontFace(this.gl.CW);
      this.gl.enable(this.gl.BLEND);
      this.gl.colorMask(true, true, true, true);
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, null);
      this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, null);
      if (this.getAnisotropy() > 0 && this._extension) {
        for (let i = 0; i < this._textures.size; ++i) {
          this.gl.bindTexture(this.gl.TEXTURE_2D, this._textures.get(i));
          this.gl.texParameterf(
            this.gl.TEXTURE_2D,
            this._extension.TEXTURE_MAX_ANISOTROPY_EXT,
            this.getAnisotropy()
          );
        }
      }
    }
    /**
     * Drawableのマスク用のオフスクリーンサーフェースを取得する
     *
     * @param index オフスクリーンサーフェースのインデックス
     *
     * @return マスク用のオフスクリーンサーフェース
     */
    getDrawableMaskBuffer(index) {
      return this._drawableMasks[index];
    }
    /**
     * マスクテクスチャに描画するクリッピングコンテキストをセットする
     */
    setClippingContextBufferForMask(clip) {
      this._clippingContextBufferForMask = clip;
    }
    /**
     * マスクテクスチャに描画するクリッピングコンテキストを取得する
     *
     * @return マスクテクスチャに描画するクリッピングコンテキスト
     */
    getClippingContextBufferForMask() {
      return this._clippingContextBufferForMask;
    }
    /**
     * Drawableの画面上に描画するクリッピングコンテキストをセットする
     *
     * @param clip drawableで画面上に描画するクリッピングコンテキスト
     */
    setClippingContextBufferForDrawable(clip) {
      this._clippingContextBufferForDraw = clip;
    }
    /**
     * Drawableの画面上に描画するクリッピングコンテキストを取得する
     *
     * @return Drawableの画面上に描画するクリッピングコンテキスト
     */
    getClippingContextBufferForDrawable() {
      return this._clippingContextBufferForDraw;
    }
    /**
     * offscreenで画面上に描画するクリッピングコンテキストをセットする。
     *
     * @param clip offscreenで画面上に描画するクリッピングコンテキスト
     */
    setClippingContextBufferForOffscreen(clip) {
      this._clippingContextBufferForOffscreen = clip;
    }
    /**
     * offscreenで画面上に描画するクリッピングコンテキストを取得する。
     *
     * @return offscreenで画面上に描画するクリッピングコンテキスト
     */
    getClippingContextBufferForOffscreen() {
      return this._clippingContextBufferForOffscreen;
    }
    /**
     * マスク生成時かを判定する
     *
     * @return 判定値
     */
    isGeneratingMask() {
      return this.getClippingContextBufferForMask() != null;
    }
    /**
     * glの設定
     */
    startUp(gl) {
      this.gl = gl;
      if (this._drawableClippingManager) {
        this._drawableClippingManager.setGL(gl);
      }
      if (this._offscreenClippingManager) {
        this._offscreenClippingManager.setGL(gl);
      }
      CubismShaderManager_WebGL.getInstance().setGlContext(gl);
      this._rendererProfile.setGl(gl);
      this._extension = this.gl.getExtension("EXT_texture_filter_anisotropic") || this.gl.getExtension("WEBKIT_EXT_texture_filter_anisotropic") || this.gl.getExtension("MOZ_EXT_texture_filter_anisotropic");
      if (this._model.isUsingMasking()) {
        this._drawableMasks.length = this._drawableClippingManager.getRenderTextureCount();
        for (let i = 0; i < this._drawableMasks.length; ++i) {
          const renderTarget = new CubismRenderTarget_WebGL();
          renderTarget.createRenderTarget(
            this.gl,
            this._drawableClippingManager.getClippingMaskBufferSize(),
            this._drawableClippingManager.getClippingMaskBufferSize(),
            this._currentFbo
          );
          this._drawableMasks[i] = renderTarget;
        }
      }
      if (this._model.isBlendModeEnabled()) {
        this._modelRenderTargets.length = 0;
        const createSize = 3;
        this._modelRenderTargets.length = createSize;
        for (let i = 0; i < createSize; ++i) {
          const offscreenRenderTarget = new CubismOffscreenRenderTarget_WebGL();
          offscreenRenderTarget.createRenderTarget(
            this.gl,
            this._modelRenderTargetWidth,
            this._modelRenderTargetHeight,
            this._currentFbo
          );
          this._modelRenderTargets[i] = offscreenRenderTarget;
        }
        if (this._model.isUsingMaskingForOffscreen()) {
          this._offscreenMasks.length = this._offscreenClippingManager.getRenderTextureCount();
          for (let i = 0; i < this._offscreenMasks.length; ++i) {
            const offscreenMask = new CubismRenderTarget_WebGL();
            offscreenMask.createRenderTarget(
              this.gl,
              this._offscreenClippingManager.getClippingMaskBufferSize(),
              this._offscreenClippingManager.getClippingMaskBufferSize(),
              this._currentFbo
            );
            this._offscreenMasks[i] = offscreenMask;
          }
        }
        const offscreenCount = this._model.getOffscreenCount();
        if (offscreenCount > 0) {
          this._offscreenList = new Array(
            offscreenCount
          );
          for (let offscreenIndex = 0; offscreenIndex < offscreenCount; ++offscreenIndex) {
            const offscreenRenderTarget = new CubismOffscreenRenderTarget_WebGL();
            offscreenRenderTarget.setOffscreenIndex(offscreenIndex);
            this._offscreenList[offscreenIndex] = offscreenRenderTarget;
          }
          this.setupParentOffscreens(this._model, offscreenCount);
        }
      }
      this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, this._currentFbo);
    }
    // webglコンテキスト
  };
  CubismRenderer.staticRelease = () => {
    CubismRenderer_WebGL.doStaticRelease();
  };
  var Live2DCubismFramework35;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismClippingContext = CubismClippingContext_WebGL;
    Live2DCubismFramework51.CubismClippingManager_WebGL = CubismClippingManager_WebGL;
    Live2DCubismFramework51.CubismRenderer_WebGL = CubismRenderer_WebGL;
  })(Live2DCubismFramework35 || (Live2DCubismFramework35 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/model/cubismmoc.ts
  var CubismMoc = class _CubismMoc {
    /**
     * Mocデータの作成
     */
    static create(mocBytes, shouldCheckMocConsistency) {
      let cubismMoc = null;
      if (shouldCheckMocConsistency) {
        const consistency = this.hasMocConsistency(mocBytes);
        if (!consistency) {
          CubismLogError(`Inconsistent MOC3.`);
          return cubismMoc;
        }
      }
      const moc = Live2DCubismCore.Moc.fromArrayBuffer(mocBytes);
      if (moc) {
        cubismMoc = new _CubismMoc(moc);
        cubismMoc._mocVersion = Live2DCubismCore.Version.csmGetMocVersion(mocBytes);
      }
      return cubismMoc;
    }
    /**
     * Mocデータを削除
     *
     * Mocデータを削除する
     */
    static delete(moc) {
      moc._moc._release();
      moc._moc = null;
      moc = null;
    }
    /**
     * モデルを作成する
     *
     * @return Mocデータから作成されたモデル
     */
    createModel() {
      let cubismModel = null;
      const model = Live2DCubismCore.Model.fromMoc(
        this._moc
      );
      if (model) {
        cubismModel = new CubismModel(model);
        cubismModel.initialize();
        ++this._modelCount;
      }
      return cubismModel;
    }
    /**
     * モデルを削除する
     */
    deleteModel(model) {
      if (model != null) {
        model.release();
        model = null;
        --this._modelCount;
      }
    }
    /**
     * コンストラクタ
     */
    constructor(moc) {
      this._moc = moc;
      this._modelCount = 0;
      this._mocVersion = 0;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      CSM_ASSERT(this._modelCount == 0);
      this._moc._release();
      this._moc = null;
    }
    /**
     * 最新の.moc3 Versionを取得
     */
    getLatestMocVersion() {
      return Live2DCubismCore.Version.csmGetLatestMocVersion();
    }
    /**
     * 読み込んだモデルの.moc3 Versionを取得
     */
    getMocVersion() {
      return this._mocVersion;
    }
    /**
     * Mocファイルのbufferから.moc3 Versionを取得
     * @param mocBytes Mocファイルのバイト配列
     * @returns .moc3 Version番号
     */
    static getMocVersionFromBuffer(mocBytes) {
      return Live2DCubismCore.Version.csmGetMocVersion(mocBytes);
    }
    /**
     * .moc3 の整合性を検証する
     */
    static hasMocConsistency(mocBytes) {
      const isConsistent = Live2DCubismCore.Moc.prototype.hasMocConsistency(mocBytes);
      return isConsistent === 1 ? true : false;
    }
    // 読み込んだモデルの.moc3 Version
  };
  var Live2DCubismFramework36;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismMoc = CubismMoc;
  })(Live2DCubismFramework36 || (Live2DCubismFramework36 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/model/cubismmodeluserdatajson.ts
  var Meta3 = "Meta";
  var UserDataCount2 = "UserDataCount";
  var TotalUserDataSize2 = "TotalUserDataSize";
  var UserData2 = "UserData";
  var Target2 = "Target";
  var Id4 = "Id";
  var Value7 = "Value";
  var CubismModelUserDataJson = class {
    /**
     * コンストラクタ
     * @param buffer    userdata3.jsonが読み込まれているバッファ
     * @param size      バッファのサイズ
     */
    constructor(buffer, size) {
      this._json = CubismJson.create(buffer, size);
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      CubismJson.delete(this._json);
    }
    /**
     * ユーザーデータ個数の取得
     * @return ユーザーデータの個数
     */
    getUserDataCount() {
      return this._json.getRoot().getValueByString(Meta3).getValueByString(UserDataCount2).toInt();
    }
    /**
     * ユーザーデータ総文字列数の取得
     *
     * @return ユーザーデータ総文字列数
     */
    getTotalUserDataSize() {
      return this._json.getRoot().getValueByString(Meta3).getValueByString(TotalUserDataSize2).toInt();
    }
    /**
     * ユーザーデータのタイプの取得
     *
     * @return ユーザーデータのタイプ
     */
    getUserDataTargetType(i) {
      return this._json.getRoot().getValueByString(UserData2).getValueByIndex(i).getValueByString(Target2).getRawString();
    }
    /**
     * ユーザーデータのターゲットIDの取得
     *
     * @param i インデックス
     * @return ユーザーデータターゲットID
     */
    getUserDataId(i) {
      return CubismFramework.getIdManager().getId(
        this._json.getRoot().getValueByString(UserData2).getValueByIndex(i).getValueByString(Id4).getRawString()
      );
    }
    /**
     * ユーザーデータの文字列の取得
     *
     * @param i インデックス
     * @return ユーザーデータ
     */
    getUserDataValue(i) {
      return this._json.getRoot().getValueByString(UserData2).getValueByIndex(i).getValueByString(Value7).getRawString();
    }
  };
  var Live2DCubismFramework37;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismModelUserDataJson = CubismModelUserDataJson;
  })(Live2DCubismFramework37 || (Live2DCubismFramework37 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/model/cubismmodeluserdata.ts
  var ArtMesh = "ArtMesh";
  var CubismModelUserDataNode = class {
    // ユーザーデータ
  };
  var CubismModelUserData = class _CubismModelUserData {
    /**
     * インスタンスの作成
     *
     * @param buffer    userdata3.jsonが読み込まれているバッファ
     * @param size      バッファのサイズ
     * @return 作成されたインスタンス
     */
    static create(buffer, size) {
      const ret = new _CubismModelUserData();
      ret.parseUserData(buffer, size);
      return ret;
    }
    /**
     * インスタンスを破棄する
     *
     * @param modelUserData 破棄するインスタンス
     */
    static delete(modelUserData) {
      if (modelUserData != null) {
        modelUserData.release();
        modelUserData = null;
      }
    }
    /**
     * ArtMeshのユーザーデータのリストの取得
     *
     * @return ユーザーデータリスト
     */
    getArtMeshUserDatas() {
      return this._artMeshUserDataNode;
    }
    /**
     * userdata3.jsonのパース
     *
     * @param buffer    userdata3.jsonが読み込まれているバッファ
     * @param size      バッファのサイズ
     */
    parseUserData(buffer, size) {
      let json = new CubismModelUserDataJson(
        buffer,
        size
      );
      if (!json) {
        json.release();
        json = void 0;
        return;
      }
      const typeOfArtMesh = CubismFramework.getIdManager().getId(ArtMesh);
      const nodeCount = json.getUserDataCount();
      let dstIndex = this._userDataNodes.length;
      this._userDataNodes.length = nodeCount;
      for (let i = 0; i < nodeCount; i++) {
        const addNode = new CubismModelUserDataNode();
        addNode.targetId = json.getUserDataId(i);
        addNode.targetType = CubismFramework.getIdManager().getId(
          json.getUserDataTargetType(i)
        );
        addNode.value = json.getUserDataValue(i);
        this._userDataNodes[dstIndex++] = addNode;
        if (addNode.targetType == typeOfArtMesh) {
          this._artMeshUserDataNode.push(addNode);
        }
      }
      json.release();
      json = void 0;
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._userDataNodes = new Array();
      this._artMeshUserDataNode = new Array();
    }
    /**
     * デストラクタ相当の処理
     *
     * ユーザーデータ構造体配列を解放する
     */
    release() {
      for (let i = 0; i < this._userDataNodes.length; ++i) {
        this._userDataNodes[i] = null;
      }
      this._userDataNodes = null;
    }
    // 閲覧リストの保持
  };
  var Live2DCubismFramework38;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismModelUserData = CubismModelUserData;
    Live2DCubismFramework51.CubismModelUserDataNode = CubismModelUserDataNode;
  })(Live2DCubismFramework38 || (Live2DCubismFramework38 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/model/cubismusermodel.ts
  var CubismUserModel = class _CubismUserModel {
    /**
     * 初期化状態の取得
     *
     * 初期化されている状態か？
     *
     * @return true     初期化されている
     * @return false    初期化されていない
     */
    isInitialized() {
      return this._initialized;
    }
    /**
     * 初期化状態の設定
     *
     * 初期化状態を設定する。
     *
     * @param v 初期化状態
     */
    setInitialized(v) {
      this._initialized = v;
    }
    /**
     * 更新状態の取得
     *
     * 更新されている状態か？
     *
     * @return true     更新されている
     * @return false    更新されていない
     */
    isUpdating() {
      return this._updating;
    }
    /**
     * 更新状態の設定
     *
     * 更新状態を設定する
     *
     * @param v 更新状態
     */
    setUpdating(v) {
      this._updating = v;
    }
    /**
     * マウスドラッグ情報の設定
     *
     * @param ドラッグしているカーソルのX位置
     * @param ドラッグしているカーソルのY位置
     */
    setDragging(x, y) {
      this._dragManager.set(x, y);
    }
    /**
     * モデル行列を取得する
     * @return モデル行列
     */
    getModelMatrix() {
      return this._modelMatrix;
    }
    /**
     * モデルを描画したバッファを設定する
     *
     * @param width モデルを描画したバッファの幅
     * @param height モデルを描画したバッファの高さ
     */
    setRenderTargetSize(width, height) {
      if (this._renderer) {
        this._renderer.setRenderTargetSize(width, height);
      }
    }
    /**
     * 不透明度の設定
     *
     * @param a 不透明度
     */
    setOpacity(a) {
      this._opacity = a;
    }
    /**
     * 不透明度の取得
     *
     * @return 不透明度
     */
    getOpacity() {
      return this._opacity;
    }
    /**
     * モデルデータを読み込む
     *
     * @param buffer    moc3ファイルが読み込まれているバッファ
     */
    loadModel(buffer, shouldCheckMocConsistency = false) {
      this._moc = CubismMoc.create(buffer, shouldCheckMocConsistency);
      if (this._moc == null) {
        CubismLogError("Failed to CubismMoc.create().");
        return;
      }
      this._model = this._moc.createModel();
      if (this._model == null) {
        CubismLogError("Failed to CreateModel().");
        return;
      }
      this._model.saveParameters();
      this._modelMatrix = new CubismModelMatrix(
        this._model.getCanvasWidth(),
        this._model.getCanvasHeight()
      );
    }
    /**
     * モーションデータを読み込む
     * @param buffer motion3.jsonファイルが読み込まれているバッファ
     * @param size バッファのサイズ
     * @param name モーションの名前
     * @param onFinishedMotionHandler モーション再生終了時に呼び出されるコールバック関数
     * @param onBeganMotionHandler モーション再生開始時に呼び出されるコールバック関数
     * @param modelSetting モデル設定
     * @param group モーショングループ名
     * @param index モーションインデックス
     * @param shouldCheckMotionConsistency motion3.json整合性チェックするかどうか
     * @return モーションクラス
     */
    loadMotion(buffer, size, name, onFinishedMotionHandler, onBeganMotionHandler, modelSetting, group, index, shouldCheckMotionConsistency = false) {
      if (buffer == null || size == 0) {
        CubismLogError("Failed to loadMotion().");
        return null;
      }
      const motion = CubismMotion.create(
        buffer,
        size,
        onFinishedMotionHandler,
        onBeganMotionHandler,
        shouldCheckMotionConsistency
      );
      if (motion == null) {
        CubismLogError(`Failed to create motion from buffer in LoadMotion()`);
        return null;
      }
      if (modelSetting) {
        const fadeInTime = modelSetting.getMotionFadeInTimeValue(
          group,
          index
        );
        if (fadeInTime >= 0) {
          motion.setFadeInTime(fadeInTime);
        }
        const fadeOutTime = modelSetting.getMotionFadeOutTimeValue(group, index);
        if (fadeOutTime >= 0) {
          motion.setFadeOutTime(fadeOutTime);
        }
      }
      return motion;
    }
    /**
     * 表情データの読み込み
     * @param buffer expファイルが読み込まれているバッファ
     * @param size バッファのサイズ
     * @param name 表情の名前
     */
    loadExpression(buffer, size, name) {
      if (buffer == null || size == 0) {
        CubismLogError("Failed to loadExpression().");
        return null;
      }
      return CubismExpressionMotion.create(buffer, size);
    }
    /**
     * ポーズデータの読み込み
     * @param buffer pose3.jsonが読み込まれているバッファ
     * @param size バッファのサイズ
     */
    loadPose(buffer, size) {
      if (buffer == null || size == 0) {
        CubismLogError("Failed to loadPose().");
        return;
      }
      this._pose = CubismPose.create(buffer, size);
    }
    /**
     * モデルに付属するユーザーデータを読み込む
     * @param buffer userdata3.jsonが読み込まれているバッファ
     * @param size バッファのサイズ
     */
    loadUserData(buffer, size) {
      if (buffer == null || size == 0) {
        CubismLogError("Failed to loadUserData().");
        return;
      }
      this._modelUserData = CubismModelUserData.create(buffer, size);
    }
    /**
     * 物理演算データの読み込み
     * @param buffer  physics3.jsonが読み込まれているバッファ
     * @param size    バッファのサイズ
     */
    loadPhysics(buffer, size) {
      if (buffer == null || size == 0) {
        CubismLogError("Failed to loadPhysics().");
        return;
      }
      this._physics = CubismPhysics.create(buffer, size);
    }
    /**
     * 当たり判定の取得
     * @param drawableId 検証したいDrawableのID
     * @param pointX X位置
     * @param pointY Y位置
     * @return true ヒットしている
     * @return false ヒットしていない
     */
    isHit(drawableId, pointX, pointY) {
      const drawIndex = this._model.getDrawableIndex(drawableId);
      if (drawIndex < 0) {
        return false;
      }
      const count = this._model.getDrawableVertexCount(drawIndex);
      const vertices = this._model.getDrawableVertices(drawIndex);
      let left = vertices[0];
      let right = vertices[0];
      let top = vertices[1];
      let bottom = vertices[1];
      for (let j = 1; j < count; ++j) {
        const x = vertices[Constant.vertexOffset + j * Constant.vertexStep];
        const y = vertices[Constant.vertexOffset + j * Constant.vertexStep + 1];
        if (x < left) {
          left = x;
        }
        if (x > right) {
          right = x;
        }
        if (y < top) {
          top = y;
        }
        if (y > bottom) {
          bottom = y;
        }
      }
      const tx = this._modelMatrix.invertTransformX(pointX);
      const ty = this._modelMatrix.invertTransformY(pointY);
      return left <= tx && tx <= right && top <= ty && ty <= bottom;
    }
    /**
     * モデルの取得
     * @return モデル
     */
    getModel() {
      return this._model;
    }
    /**
     * 読み込めないMocファイルの.moc3 Versionを取得
     * @param mocBytes 読み込めないMocファイルのバイト配列
     * @returns .moc3 Version番号
     */
    getMocVersionFromBuffer(mocBytes) {
      return CubismMoc.getMocVersionFromBuffer(mocBytes);
    }
    /**
     * レンダラの取得
     * @return レンダラ
     */
    getRenderer() {
      return this._renderer;
    }
    /**
     * レンダラを作成して初期化を実行する
     * @param width レンダリングする幅
     * @param height レンダリングする高さ
     * @param maskBufferCount バッファの生成数
     */
    createRenderer(width, height, maskBufferCount = 1) {
      if (this._renderer) {
        this.deleteRenderer();
      }
      this._renderer = new CubismRenderer_WebGL(width, height);
      this._renderer.initialize(this._model, maskBufferCount);
    }
    /**
     * レンダラの解放
     */
    deleteRenderer() {
      if (this._renderer != null) {
        this._renderer.release();
        this._renderer = null;
      }
    }
    /**
     * イベント発火時の標準処理
     *
     * Eventが再生処理時にあった場合の処理をする。
     * 継承で上書きすることを想定している。
     * 上書きしない場合はログ出力をする。
     *
     * @param eventValue 発火したイベントの文字列データ
     */
    motionEventFired(eventValue) {
      CubismLogInfo("{0}", eventValue);
    }
    /**
     * イベント用のコールバック
     *
     * CubismMotionQueueManagerにイベント用に登録するためのCallback。
     * CubismUserModelの継承先のEventFiredを呼ぶ。
     *
     * @param caller 発火したイベントを管理していたモーションマネージャー、比較用
     * @param eventValue 発火したイベントの文字列データ
     * @param customData CubismUserModelを継承したインスタンスを想定
     */
    static cubismDefaultMotionEventCallback(caller, eventValue, customData) {
      const model = customData;
      if (model != null) {
        model.motionEventFired(eventValue);
      }
    }
    /**
     * コンストラクタ
     */
    constructor() {
      this._moc = null;
      this._model = null;
      this._motionManager = null;
      this._expressionManager = null;
      this._eyeBlink = null;
      this._breath = null;
      this._modelMatrix = null;
      this._pose = null;
      this._dragManager = null;
      this._physics = null;
      this._modelUserData = null;
      this._initialized = false;
      this._updating = false;
      this._opacity = 1;
      this._mocConsistency = false;
      this._debugMode = false;
      this._renderer = null;
      this._motionManager = new CubismMotionManager();
      this._motionManager.setEventCallback(
        _CubismUserModel.cubismDefaultMotionEventCallback,
        this
      );
      this._expressionManager = new CubismExpressionMotionManager();
      this._dragManager = new CubismTargetPoint();
    }
    /**
     * デストラクタに相当する処理
     */
    release() {
      if (this._motionManager != null) {
        this._motionManager.release();
        this._motionManager = null;
      }
      if (this._expressionManager != null) {
        this._expressionManager.release();
        this._expressionManager = null;
      }
      if (this._moc != null) {
        this._moc.deleteModel(this._model);
        this._moc.release();
        this._moc = null;
      }
      this._modelMatrix = null;
      CubismPose.delete(this._pose);
      CubismEyeBlink.delete(this._eyeBlink);
      CubismBreath.delete(this._breath);
      this._dragManager = null;
      CubismPhysics.delete(this._physics);
      CubismModelUserData.delete(this._modelUserData);
      this.deleteRenderer();
    }
    // レンダラ
  };
  var Live2DCubismFramework39;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismUserModel = CubismUserModel;
  })(Live2DCubismFramework39 || (Live2DCubismFramework39 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/icubismupdater.ts
  var CubismUpdateOrder = ((CubismUpdateOrder2) => {
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_EyeBlink"] = 200] = "CubismUpdateOrder_EyeBlink";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_Expression"] = 300] = "CubismUpdateOrder_Expression";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_Drag"] = 400] = "CubismUpdateOrder_Drag";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_Breath"] = 500] = "CubismUpdateOrder_Breath";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_Physics"] = 600] = "CubismUpdateOrder_Physics";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_LipSync"] = 700] = "CubismUpdateOrder_LipSync";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_Pose"] = 800] = "CubismUpdateOrder_Pose";
    CubismUpdateOrder2[CubismUpdateOrder2["CubismUpdateOrder_Max"] = Number.MAX_SAFE_INTEGER] = "CubismUpdateOrder_Max";
    return CubismUpdateOrder2;
  })(CubismUpdateOrder || {});
  var ICubismUpdater = class {
    /**
     * Constructor
     */
    constructor(executionOrder = 0) {
      this._changeListeners = [];
      this._executionOrder = executionOrder;
    }
    /**
     * Comparison function used when sorting ICubismUpdater objects.
     *
     * @param left The first ICubismUpdater object to be compared.
     * @param right The second ICubismUpdater object to be compared.
     *
     * @return negative if left should be placed before right,
     *         positive if right should be placed before left,
     *         zero if they are equal.
     */
    static sortFunction(left, right) {
      if (!left || !right) {
        if (!left && !right) return 0;
        if (!left) return 1;
        if (!right) return -1;
      }
      return left.getExecutionOrder() - right.getExecutionOrder();
    }
    getExecutionOrder() {
      return this._executionOrder;
    }
    setExecutionOrder(executionOrder) {
      if (this._executionOrder !== executionOrder) {
        this._executionOrder = executionOrder;
        this.notifyChangeListeners();
      }
    }
    /**
     * Adds a listener to be notified when this updater's properties change.
     *
     * @param listener The listener to add
     */
    addChangeListener(listener) {
      if (listener && this._changeListeners.indexOf(listener) === -1) {
        this._changeListeners.push(listener);
      }
    }
    /**
     * Removes a listener from the notification list.
     *
     * @param listener The listener to remove
     */
    removeChangeListener(listener) {
      const index = this._changeListeners.indexOf(listener);
      if (index >= 0) {
        this._changeListeners.splice(index, 1);
      }
    }
    /**
     * Notifies all registered listeners that this updater has changed.
     */
    notifyChangeListeners() {
      for (const listener of this._changeListeners) {
        listener.onUpdaterChanged(this);
      }
    }
  };
  var Live2DCubismFramework40;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.ICubismUpdater = ICubismUpdater;
  })(Live2DCubismFramework40 || (Live2DCubismFramework40 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismupdatescheduler.ts
  var CubismUpdateScheduler = class {
    /**
     * Constructor
     */
    constructor() {
      this._cubismUpdatableList = [];
      this._needsSort = false;
    }
    /**
     * Destructor equivalent - releases all updaters and removes listeners
     */
    release() {
      for (const updater of this._cubismUpdatableList) {
        if (updater) {
          updater.removeChangeListener(this);
        }
      }
      this._cubismUpdatableList.length = 0;
    }
    /**
     * Adds ICubismUpdater to the update list.
     * The list will be automatically sorted by execution order before the next update.
     *
     * @param updatable The ICubismUpdater instance to be added.
     */
    addUpdatableList(updatable) {
      if (!updatable) {
        return;
      }
      if (this.hasUpdatable(updatable)) {
        return;
      }
      this._cubismUpdatableList.push(updatable);
      updatable.addChangeListener(this);
      this._needsSort = true;
    }
    /**
     * Removes ICubismUpdater from the update list.
     *
     * @param updatable The ICubismUpdater instance to be removed.
     * @return true if the updater was found and removed, false otherwise.
     */
    removeUpdatableList(updatable) {
      if (!updatable) {
        return false;
      }
      const index = this._cubismUpdatableList.indexOf(updatable);
      if (index >= 0) {
        this._cubismUpdatableList.splice(index, 1);
        updatable.removeChangeListener(this);
        return true;
      }
      return false;
    }
    /**
     * Sorts the update list using the ICubismUpdater sort function.
     */
    sortUpdatableList() {
      this._cubismUpdatableList.sort(ICubismUpdater.sortFunction);
      this._needsSort = false;
    }
    /**
     * Updates every element in the list.
     * The list is automatically sorted by execution order before execution.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      if (this._needsSort) {
        this.sortUpdatableList();
      }
      for (let i = 0; i < this._cubismUpdatableList.length; ++i) {
        const updater = this._cubismUpdatableList[i];
        if (updater) {
          updater.onLateUpdate(model, deltaTimeSeconds);
        }
      }
    }
    /**
     * Gets the number of updaters in the list.
     *
     * @return Number of updaters
     */
    getUpdatableCount() {
      return this._cubismUpdatableList.length;
    }
    /**
     * Gets the updater at the specified index.
     *
     * @param index Index of the updater to retrieve
     * @return The updater at the specified index, or null if index is out of bounds
     */
    getUpdatable(index) {
      if (index < 0 || index >= this._cubismUpdatableList.length) {
        return null;
      }
      return this._cubismUpdatableList[index];
    }
    /**
     * Checks if the specified updater exists in the list.
     *
     * @param updatable The updater to check for
     * @return true if the updater exists in the list, false otherwise
     */
    hasUpdatable(updatable) {
      return this._cubismUpdatableList.indexOf(updatable) >= 0;
    }
    /**
     * Clears all updaters from the list.
     */
    clearUpdatableList() {
      for (const updater of this._cubismUpdatableList) {
        if (updater) {
          updater.removeChangeListener(this);
        }
      }
      this._cubismUpdatableList.length = 0;
      this._needsSort = false;
    }
    /**
     * Called when an updater's execution order has changed.
     * Marks the list for re-sorting.
     *
     * @param updater The updater that was changed
     */
    onUpdaterChanged(updater) {
      this._needsSort = true;
    }
  };
  var Live2DCubismFramework41;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismUpdateScheduler = CubismUpdateScheduler;
  })(Live2DCubismFramework41 || (Live2DCubismFramework41 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismbreathupdater.ts
  var CubismBreathUpdater = class extends ICubismUpdater {
    constructor(breath, executionOrder) {
      super(executionOrder != null ? executionOrder : 500 /* CubismUpdateOrder_Breath */);
      this._breath = breath;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      this._breath.updateParameters(model, deltaTimeSeconds);
    }
  };
  var Live2DCubismFramework42;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismBreathUpdater = CubismBreathUpdater;
  })(Live2DCubismFramework42 || (Live2DCubismFramework42 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismlookupdater.ts
  var CubismLookUpdater = class extends ICubismUpdater {
    constructor(look, dragManager, executionOrder) {
      super(executionOrder != null ? executionOrder : 400 /* CubismUpdateOrder_Drag */);
      this._look = look;
      this._dragManager = dragManager;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      this._dragManager.update(deltaTimeSeconds);
      const dragX = this._dragManager.getX();
      const dragY = this._dragManager.getY();
      this._look.updateParameters(model, dragX, dragY);
    }
  };
  var Live2DCubismFramework43;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismLookUpdater = CubismLookUpdater;
  })(Live2DCubismFramework43 || (Live2DCubismFramework43 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismeyeblinkupdater.ts
  var CubismEyeBlinkUpdater = class extends ICubismUpdater {
    constructor(motionUpdated, eyeBlink, executionOrder) {
      super(executionOrder != null ? executionOrder : 200 /* CubismUpdateOrder_EyeBlink */);
      this._motionUpdated = motionUpdated;
      this._eyeBlink = eyeBlink;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      if (!this._motionUpdated()) {
        this._eyeBlink.updateParameters(model, deltaTimeSeconds);
      }
    }
  };
  var Live2DCubismFramework44;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismEyeBlinkUpdater = CubismEyeBlinkUpdater;
  })(Live2DCubismFramework44 || (Live2DCubismFramework44 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismexpressionupdater.ts
  var CubismExpressionUpdater = class extends ICubismUpdater {
    constructor(expressionManager, executionOrder) {
      super(executionOrder != null ? executionOrder : 300 /* CubismUpdateOrder_Expression */);
      this._expressionManager = expressionManager;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      this._expressionManager.updateMotion(model, deltaTimeSeconds);
    }
  };
  var Live2DCubismFramework45;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismExpressionUpdater = CubismExpressionUpdater;
  })(Live2DCubismFramework45 || (Live2DCubismFramework45 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismphysicsupdater.ts
  var CubismPhysicsUpdater = class extends ICubismUpdater {
    constructor(physics, executionOrder) {
      super(executionOrder != null ? executionOrder : 600 /* CubismUpdateOrder_Physics */);
      this._physics = physics;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      this._physics.evaluate(model, deltaTimeSeconds);
    }
  };
  var Live2DCubismFramework46;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismPhysicsUpdater = CubismPhysicsUpdater;
  })(Live2DCubismFramework46 || (Live2DCubismFramework46 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismposeupdater.ts
  var CubismPoseUpdater = class extends ICubismUpdater {
    constructor(pose, executionOrder) {
      super(executionOrder != null ? executionOrder : 800 /* CubismUpdateOrder_Pose */);
      this._pose = pose;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      this._pose.updateParameters(model, deltaTimeSeconds);
    }
  };
  var Live2DCubismFramework47;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismPoseUpdater = CubismPoseUpdater;
  })(Live2DCubismFramework47 || (Live2DCubismFramework47 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/cubismlipsyncupdater.ts
  var CubismLipSyncUpdater = class extends ICubismUpdater {
    constructor(lipSyncIds, audioProvider, executionOrder) {
      super(executionOrder != null ? executionOrder : 700 /* CubismUpdateOrder_LipSync */);
      this._lipSyncIds = [...lipSyncIds];
      this._audioProvider = audioProvider;
    }
    /**
     * Update process.
     *
     * @param model Model to update
     * @param deltaTimeSeconds Delta time in seconds.
     */
    onLateUpdate(model, deltaTimeSeconds) {
      if (!model) {
        return;
      }
      if (this._audioProvider) {
        const updateSuccessful = this._audioProvider.update(deltaTimeSeconds);
        if (updateSuccessful) {
          const lipSyncValue = this._audioProvider.getParameter();
          for (let i = 0; i < this._lipSyncIds.length; i++) {
            model.addParameterValueById(this._lipSyncIds[i], lipSyncValue);
          }
        }
      }
    }
    /**
     * Set audio parameter provider.
     *
     * @param audioProvider Audio parameter provider to set
     */
    setAudioProvider(audioProvider) {
      this._audioProvider = audioProvider;
    }
    /**
     * Get audio parameter provider.
     *
     * @return Current audio parameter provider
     */
    getAudioProvider() {
      return this._audioProvider;
    }
  };
  var Live2DCubismFramework48;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismLipSyncUpdater = CubismLipSyncUpdater;
  })(Live2DCubismFramework48 || (Live2DCubismFramework48 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/motion/iparameterprovider.ts
  var IParameterProvider = class {
    /**
     * Constructor
     */
    constructor() {
    }
  };
  var Live2DCubismFramework49;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.IParameterProvider = IParameterProvider;
  })(Live2DCubismFramework49 || (Live2DCubismFramework49 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappwavfilehandler.ts
  var LAppWavFileHandler = class extends IParameterProvider {
    constructor() {
      super();
      this.loadFiletoBytes = (arrayBuffer, length) => {
        this._byteReader._fileByte = arrayBuffer;
        this._byteReader._fileDataView = new DataView(this._byteReader._fileByte);
        this._byteReader._fileSize = length;
      };
      this._pcmData = null;
      this._userTimeSeconds = 0;
      this._lastRms = 0;
      this._sampleOffset = 0;
      this._wavFileInfo = new WavFileInfo();
      this._byteReader = new ByteReader();
    }
    update(deltaTimeSeconds) {
      let goalOffset;
      let rms;
      if (this._pcmData == null || this._sampleOffset >= this._wavFileInfo._samplesPerChannel) {
        this._lastRms = 0;
        return false;
      }
      const actualDeltaTime = deltaTimeSeconds != null ? deltaTimeSeconds : 1 / 60;
      this._userTimeSeconds += actualDeltaTime;
      goalOffset = Math.floor(
        this._userTimeSeconds * this._wavFileInfo._samplingRate
      );
      if (goalOffset > this._wavFileInfo._samplesPerChannel) {
        goalOffset = this._wavFileInfo._samplesPerChannel;
      }
      rms = 0;
      for (let channelCount = 0; channelCount < this._wavFileInfo._numberOfChannels; channelCount++) {
        for (let sampleCount = this._sampleOffset; sampleCount < goalOffset; sampleCount++) {
          const pcm = this._pcmData[channelCount][sampleCount];
          rms += pcm * pcm;
        }
      }
      rms = Math.sqrt(
        rms / (this._wavFileInfo._numberOfChannels * (goalOffset - this._sampleOffset))
      );
      this._lastRms = rms;
      this._sampleOffset = goalOffset;
      return true;
    }
    start(filePath) {
      this._sampleOffset = 0;
      this._userTimeSeconds = 0;
      this._lastRms = 0;
      this.loadWavFile(filePath);
    }
    /**
     * Get parameter value for lip sync.
     *
     * @return RMS value from audio
     */
    getParameter() {
      return this.getRms();
    }
    getRms() {
      return this._lastRms;
    }
    loadWavFile(filePath) {
      return new Promise((resolveValue) => {
        let ret = false;
        if (this._pcmData != null) {
          this.releasePcmData();
        }
        const asyncFileLoad = async () => {
          return fetch(filePath).then((responce) => {
            return responce.arrayBuffer();
          });
        };
        const asyncWavFileManager = (async () => {
          this._byteReader._fileByte = await asyncFileLoad();
          this._byteReader._fileDataView = new DataView(
            this._byteReader._fileByte
          );
          this._byteReader._fileSize = this._byteReader._fileByte.byteLength;
          this._byteReader._readOffset = 0;
          if (this._byteReader._fileByte == null || this._byteReader._fileSize < 4) {
            resolveValue(false);
            return;
          }
          this._wavFileInfo._fileName = filePath;
          try {
            if (!this._byteReader.getCheckSignature("RIFF")) {
              ret = false;
              throw new Error('Cannot find Signeture "RIFF".');
            }
            this._byteReader.get32LittleEndian();
            if (!this._byteReader.getCheckSignature("WAVE")) {
              ret = false;
              throw new Error('Cannot find Signeture "WAVE".');
            }
            if (!this._byteReader.getCheckSignature("fmt ")) {
              ret = false;
              throw new Error('Cannot find Signeture "fmt".');
            }
            const fmtChunkSize = this._byteReader.get32LittleEndian();
            if (this._byteReader.get16LittleEndian() != 1) {
              ret = false;
              throw new Error("File is not linear PCM.");
            }
            this._wavFileInfo._numberOfChannels = this._byteReader.get16LittleEndian();
            this._wavFileInfo._samplingRate = this._byteReader.get32LittleEndian();
            this._byteReader.get32LittleEndian();
            this._byteReader.get16LittleEndian();
            this._wavFileInfo._bitsPerSample = this._byteReader.get16LittleEndian();
            if (fmtChunkSize > 16) {
              this._byteReader._readOffset += fmtChunkSize - 16;
            }
            while (!this._byteReader.getCheckSignature("data") && this._byteReader._readOffset < this._byteReader._fileSize) {
              this._byteReader._readOffset += this._byteReader.get32LittleEndian() + 4;
            }
            if (this._byteReader._readOffset >= this._byteReader._fileSize) {
              ret = false;
              throw new Error('Cannot find "data" Chunk.');
            }
            {
              const dataChunkSize = this._byteReader.get32LittleEndian();
              this._wavFileInfo._samplesPerChannel = dataChunkSize * 8 / (this._wavFileInfo._bitsPerSample * this._wavFileInfo._numberOfChannels);
            }
            this._pcmData = new Array(this._wavFileInfo._numberOfChannels);
            for (let channelCount = 0; channelCount < this._wavFileInfo._numberOfChannels; channelCount++) {
              this._pcmData[channelCount] = new Float32Array(
                this._wavFileInfo._samplesPerChannel
              );
            }
            for (let sampleCount = 0; sampleCount < this._wavFileInfo._samplesPerChannel; sampleCount++) {
              for (let channelCount = 0; channelCount < this._wavFileInfo._numberOfChannels; channelCount++) {
                this._pcmData[channelCount][sampleCount] = this.getPcmSample();
              }
            }
            ret = true;
            resolveValue(ret);
          } catch (e) {
            console.log(e);
          }
        })().then(() => {
          resolveValue(ret);
        });
      });
    }
    getPcmSample() {
      let pcm32;
      switch (this._wavFileInfo._bitsPerSample) {
        case 8:
          pcm32 = this._byteReader.get8() - 128;
          pcm32 <<= 24;
          break;
        case 16:
          pcm32 = this._byteReader.get16LittleEndian() << 16;
          break;
        case 24:
          pcm32 = this._byteReader.get24LittleEndian() << 8;
          break;
        default:
          pcm32 = 0;
          break;
      }
      return pcm32 / 2147483647;
    }
    /**
     * 指定したチャンネルから音声サンプルの配列を取得する
     *
     * @param usechannel 利用するチャンネル
     * @return 指定したチャンネルの音声サンプルの配列
     */
    getPcmDataChannel(usechannel) {
      if (!this._pcmData || !(usechannel < this._pcmData.length)) {
        return null;
      }
      return Float32Array.from(this._pcmData[usechannel]);
    }
    /**
     * 音声のサンプリング周波数を取得する。
     *
     * @return 音声のサンプリング周波数
     */
    getWavSamplingRate() {
      if (!this._wavFileInfo || this._wavFileInfo._samplingRate < 1) {
        return null;
      }
      return this._wavFileInfo._samplingRate;
    }
    releasePcmData() {
      for (let channelCount = 0; channelCount < this._wavFileInfo._numberOfChannels; channelCount++) {
        this._pcmData[channelCount] = null;
      }
      delete this._pcmData;
      this._pcmData = null;
    }
  };
  var WavFileInfo = class {
    constructor() {
      this._fileName = "";
      this._numberOfChannels = 0;
      this._bitsPerSample = 0;
      this._samplingRate = 0;
      this._samplesPerChannel = 0;
    }
    ///< 1チャンネルあたり総サンプル数
  };
  var ByteReader = class {
    constructor() {
      this._fileByte = null;
      this._fileDataView = null;
      this._fileSize = 0;
      this._readOffset = 0;
    }
    /**
     * @brief 8ビット読み込み
     * @return Csm::csmUint8 読み取った8ビット値
     */
    get8() {
      const ret = this._fileDataView.getUint8(this._readOffset);
      this._readOffset++;
      return ret;
    }
    /**
     * @brief 16ビット読み込み（リトルエンディアン）
     * @return Csm::csmUint16 読み取った16ビット値
     */
    get16LittleEndian() {
      const ret = this._fileDataView.getUint8(this._readOffset + 1) << 8 | this._fileDataView.getUint8(this._readOffset);
      this._readOffset += 2;
      return ret;
    }
    /**
     * @brief 24ビット読み込み（リトルエンディアン）
     * @return Csm::csmUint32 読み取った24ビット値（下位24ビットに設定）
     */
    get24LittleEndian() {
      const ret = this._fileDataView.getUint8(this._readOffset + 2) << 16 | this._fileDataView.getUint8(this._readOffset + 1) << 8 | this._fileDataView.getUint8(this._readOffset);
      this._readOffset += 3;
      return ret;
    }
    /**
     * @brief 32ビット読み込み（リトルエンディアン）
     * @return Csm::csmUint32 読み取った32ビット値
     */
    get32LittleEndian() {
      const ret = this._fileDataView.getUint8(this._readOffset + 3) << 24 | this._fileDataView.getUint8(this._readOffset + 2) << 16 | this._fileDataView.getUint8(this._readOffset + 1) << 8 | this._fileDataView.getUint8(this._readOffset);
      this._readOffset += 4;
      return ret;
    }
    /**
     * @brief シグネチャの取得と参照文字列との一致チェック
     * @param[in] reference 検査対象のシグネチャ文字列
     * @return  true    一致している
     *          false   一致していない
     */
    getCheckSignature(reference) {
      const getSignature = new Uint8Array(4);
      const referenceString = new TextEncoder().encode(reference);
      if (reference.length != 4) {
        return false;
      }
      for (let signatureOffset = 0; signatureOffset < 4; signatureOffset++) {
        getSignature[signatureOffset] = this.get8();
      }
      return getSignature[0] == referenceString[0] && getSignature[1] == referenceString[1] && getSignature[2] == referenceString[2] && getSignature[3] == referenceString[3];
    }
    ///< ファイル参照位置
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappmodel.ts
  var LAppModel = class extends CubismUserModel {
    /**
     * model3.jsonが置かれたディレクトリとファイルパスからモデルを生成する
     * @param dir
     * @param fileName
     */
    loadAssets(dir, fileName) {
      this._modelHomeDir = dir;
      console.log(`[ReamaCubism] Loading model: ${this._modelHomeDir}${fileName}`);
      cachedFetchArrayBuffer(`${this._modelHomeDir}${fileName}`).then((arrayBuffer) => {
        console.log(`[ReamaCubism] model3.json loaded (${arrayBuffer.byteLength} bytes)`);
        const setting = new CubismModelSettingJson(
          arrayBuffer,
          arrayBuffer.byteLength
        );
        this._state = 1 /* LoadModel */;
        this.setupModel(setting);
      }).catch((error) => {
        console.error(`[ReamaCubism] Failed to load model: ${this._modelHomeDir}${fileName}`, error);
        CubismLogError(`Failed to load file ${this._modelHomeDir}${fileName}`);
      });
    }
    /**
     * model3.jsonからモデルを生成する。
     * model3.jsonの記述に従ってモデル生成、モーション、物理演算などのコンポーネント生成を行う。
     *
     * @param setting ICubismModelSettingのインスタンス
     */
    setupModel(setting) {
      this._updating = true;
      this._initialized = false;
      this._modelSetting = setting;
      // Kick off parallel pre-fetches for all referenced model assets immediately
      if (this._modelSetting.getModelFileName() != "") {
        cachedFetchArrayBuffer(`${this._modelHomeDir}${this._modelSetting.getModelFileName()}`).catch(() => {});
      }
      if (this._modelSetting.getPhysicsFileName() != "") {
        cachedFetchArrayBuffer(`${this._modelHomeDir}${this._modelSetting.getPhysicsFileName()}`).catch(() => {});
      }
      const preTexCount = this._modelSetting.getTextureCount();
      for (let ti = 0; ti < preTexCount; ti++) {
        const tf = this._modelSetting.getTextureFileName(ti);
        if (tf != "") {
          cachedLoadImage(`${this._modelHomeDir}${tf}`).catch(() => {});
        }
      }
      if (this._modelSetting.getModelFileName() != "") {
        const modelFileName = this._modelSetting.getModelFileName();
        cachedFetchArrayBuffer(`${this._modelHomeDir}${modelFileName}`).catch(() => {
          CubismLogError(
            `Failed to load file ${this._modelHomeDir}${modelFileName}`
          );
          return new ArrayBuffer(0);
        }).then((arrayBuffer) => {
          this.loadModel(arrayBuffer, this._mocConsistency);
          this._state = 3 /* LoadExpression */;
          loadCubismExpression();
        });
        this._state = 2 /* WaitLoadModel */;
      } else {
        LAppPal.printMessage("Model data does not exist.");
      }
      const loadCubismExpression = () => {
        if (this._modelSetting.getExpressionCount() > 0) {
          const count = this._modelSetting.getExpressionCount();
          for (let i = 0; i < count; i++) {
            const expressionName = this._modelSetting.getExpressionName(i);
            const expressionFileName = this._modelSetting.getExpressionFileName(i);
            cachedFetchArrayBuffer(`${this._modelHomeDir}${expressionFileName}`).catch(() => {
              CubismLogError(
                `Failed to load file ${this._modelHomeDir}${expressionFileName}`
              );
              return new ArrayBuffer(0);
            }).then((arrayBuffer) => {
              const motion = this.loadExpression(
                arrayBuffer,
                arrayBuffer.byteLength,
                expressionName
              );
              if (this._expressions.get(expressionName) != null) {
                ACubismMotion.delete(this._expressions.get(expressionName));
                this._expressions.set(expressionName, null);
              }
              this._expressions.set(expressionName, motion);
              this._expressionCount++;
              if (this._expressionCount >= count) {
                if (this._expressionManager != null) {
                  const expressionUpdater = new CubismExpressionUpdater(
                    this._expressionManager
                  );
                  this._updateScheduler.addUpdatableList(expressionUpdater);
                }
                this._state = 5 /* LoadPhysics */;
                loadCubismPhysics();
              }
            });
          }
          this._state = 4 /* WaitLoadExpression */;
        } else {
          this._state = 5 /* LoadPhysics */;
          loadCubismPhysics();
        }
      };
      const loadCubismPhysics = () => {
        if (this._modelSetting.getPhysicsFileName() != "") {
          const physicsFileName = this._modelSetting.getPhysicsFileName();
          cachedFetchArrayBuffer(`${this._modelHomeDir}${physicsFileName}`).catch(() => {
            CubismLogError(
              `Failed to load file ${this._modelHomeDir}${physicsFileName}`
            );
            return new ArrayBuffer(0);
          }).then((arrayBuffer) => {
            this.loadPhysics(arrayBuffer, arrayBuffer.byteLength);
            if (this._physics) {
              const physicsUpdater = new CubismPhysicsUpdater(this._physics);
              this._updateScheduler.addUpdatableList(physicsUpdater);
            }
            this._state = 7 /* LoadPose */;
            loadCubismPose();
          });
          this._state = 6 /* WaitLoadPhysics */;
        } else {
          this._state = 7 /* LoadPose */;
          loadCubismPose();
        }
      };
      const loadCubismPose = () => {
        if (this._modelSetting.getPoseFileName() != "") {
          const poseFileName = this._modelSetting.getPoseFileName();
          fetch(`${this._modelHomeDir}${poseFileName}`).then((response) => {
            if (response.ok) {
              return response.arrayBuffer();
            } else if (response.status >= 400) {
              CubismLogError(
                `Failed to load file ${this._modelHomeDir}${poseFileName}`
              );
              return new ArrayBuffer(0);
            }
          }).then((arrayBuffer) => {
            this.loadPose(arrayBuffer, arrayBuffer.byteLength);
            if (this._pose) {
              const poseUpdater = new CubismPoseUpdater(this._pose);
              this._updateScheduler.addUpdatableList(poseUpdater);
            }
            this._state = 9 /* SetupEyeBlink */;
            setupEyeBlink();
          });
          this._state = 8 /* WaitLoadPose */;
        } else {
          this._state = 9 /* SetupEyeBlink */;
          setupEyeBlink();
        }
      };
      const setupEyeBlink = () => {
        if (this._modelSetting.getEyeBlinkParameterCount() > 0) {
          this._eyeBlink = CubismEyeBlink.create(this._modelSetting);
          const eyeBlinkUpdater = new CubismEyeBlinkUpdater(
            () => this._motionUpdated,
            this._eyeBlink
          );
          this._updateScheduler.addUpdatableList(eyeBlinkUpdater);
        }
        this._state = 10 /* SetupBreath */;
        setupBreath();
      };
      const setupBreath = () => {
        this._breath = CubismBreath.create();
        const breathParameters = [
          new BreathParameterData(this._idParamAngleX, 0, 15, 6.5345, 0.5),
          new BreathParameterData(this._idParamAngleY, 0, 8, 3.5345, 0.5),
          new BreathParameterData(this._idParamAngleZ, 0, 10, 5.5345, 0.5),
          new BreathParameterData(
            this._idParamBodyAngleX,
            0,
            4,
            15.5345,
            0.5
          ),
          new BreathParameterData(
            CubismFramework.getIdManager().getId(
              CubismDefaultParameterId.ParamBreath
            ),
            0.5,
            0.5,
            3.2345,
            1
          )
        ];
        this._breath.setParameters(breathParameters);
        const breathUpdater = new CubismBreathUpdater(this._breath);
        this._updateScheduler.addUpdatableList(breathUpdater);
        this._state = 11 /* LoadUserData */;
        loadUserData();
      };
      const loadUserData = () => {
        if (this._modelSetting.getUserDataFile() != "") {
          const userDataFile = this._modelSetting.getUserDataFile();
          fetch(`${this._modelHomeDir}${userDataFile}`).then((response) => {
            if (response.ok) {
              return response.arrayBuffer();
            } else if (response.status >= 400) {
              CubismLogError(
                `Failed to load file ${this._modelHomeDir}${userDataFile}`
              );
              return new ArrayBuffer(0);
            }
          }).then((arrayBuffer) => {
            this.loadUserData(arrayBuffer, arrayBuffer.byteLength);
            this._state = 13 /* SetupEyeBlinkIds */;
            setupEyeBlinkIds();
          });
          this._state = 12 /* WaitLoadUserData */;
        } else {
          this._state = 13 /* SetupEyeBlinkIds */;
          setupEyeBlinkIds();
        }
      };
      const setupEyeBlinkIds = () => {
        const eyeBlinkIdCount = this._modelSetting.getEyeBlinkParameterCount();
        this._eyeBlinkIds.length = eyeBlinkIdCount;
        for (let i = 0; i < eyeBlinkIdCount; ++i) {
          this._eyeBlinkIds[i] = this._modelSetting.getEyeBlinkParameterId(i);
        }
        this._state = 14 /* SetupLipSyncIds */;
        setupLipSyncIds();
      };
      const setupLipSyncIds = () => {
        const lipSyncIdCount = this._modelSetting.getLipSyncParameterCount();
        this._lipSyncIds.length = lipSyncIdCount;
        for (let i = 0; i < lipSyncIdCount; ++i) {
          this._lipSyncIds[i] = this._modelSetting.getLipSyncParameterId(i);
        }
        if (this._lipSyncIds.length > 0) {
          const lipSyncUpdater = new CubismLipSyncUpdater(
            this._lipSyncIds,
            this._wavFileHandler
          );
          this._updateScheduler.addUpdatableList(lipSyncUpdater);
        }
        this._state = 15 /* SetupLook */;
        setupLook();
      };
      const setupLook = () => {
        this._look = CubismLook.create();
        const lookParameters = [
          new LookParameterData(this._idParamAngleX, 30, 0, 0),
          new LookParameterData(this._idParamAngleY, 0, 30, 0),
          new LookParameterData(this._idParamAngleZ, 0, 0, -30),
          new LookParameterData(this._idParamBodyAngleX, 10, 0, 0),
          new LookParameterData(
            CubismFramework.getIdManager().getId(
              CubismDefaultParameterId.ParamEyeBallX
            ),
            1,
            0,
            0
          ),
          new LookParameterData(
            CubismFramework.getIdManager().getId(
              CubismDefaultParameterId.ParamEyeBallY
            ),
            0,
            1,
            0
          )
        ];
        this._look.setParameters(lookParameters);
        const lookUpdater = new CubismLookUpdater(this._look, this._dragManager);
        this._updateScheduler.addUpdatableList(lookUpdater);
        finalizeUpdaters();
      };
      const finalizeUpdaters = () => {
        this._updateScheduler.sortUpdatableList();
        this._state = 16 /* SetupLayout */;
        setupLayout();
      };
      const setupLayout = () => {
        const layout = /* @__PURE__ */ new Map();
        if (this._modelSetting == null || this._modelMatrix == null) {
          CubismLogError("Failed to setupLayout().");
          return;
        }
        this._modelSetting.getLayoutMap(layout);
        this._modelMatrix.setupFromLayout(layout);
        this._state = 17 /* LoadMotion */;
        loadCubismMotion();
      };
      const loadCubismMotion = () => {
        this._state = 18 /* WaitLoadMotion */;
        this._model.saveParameters();
        this._allMotionCount = 0;
        this._motionCount = 0;
        const group = [];
        const motionGroupCount = this._modelSetting.getMotionGroupCount();
        for (let i = 0; i < motionGroupCount; i++) {
          group[i] = this._modelSetting.getMotionGroupName(i);
          this._allMotionCount += this._modelSetting.getMotionCount(group[i]);
        }
        for (let i = 0; i < motionGroupCount; i++) {
          this.preLoadMotionGroup(group[i]);
        }
        if (motionGroupCount == 0) {
          this._state = 21 /* LoadTexture */;
          this._motionManager.stopAllMotions();
          this._updating = false;
          this._initialized = true;
          this.createRenderer(
            this._subdelegate.getCanvas().width,
            this._subdelegate.getCanvas().height
          );
          this.setupTextures();
          this.getRenderer().startUp(this._subdelegate.getGlManager().getGl());
          this.getRenderer().loadShaders(ShaderPath);
        }
      };
    }
    /**
     * テクスチャユニットにテクスチャをロードする
     */
    setupTextures() {
      const usePremultiply = true;
      if (this._state == 21 /* LoadTexture */) {
        const textureCount = this._modelSetting.getTextureCount();
        console.log(`[ReamaCubism] Loading ${textureCount} texture(s)...`);
        for (let modelTextureNumber = 0; modelTextureNumber < textureCount; modelTextureNumber++) {
          if (this._modelSetting.getTextureFileName(modelTextureNumber) == "") {
            console.log("getTextureFileName null");
            continue;
          }
          let texturePath = this._modelSetting.getTextureFileName(modelTextureNumber);
          texturePath = this._modelHomeDir + texturePath;
          console.log(`[ReamaCubism] Loading texture ${modelTextureNumber}: ${texturePath}`);
          const onLoad = (textureInfo) => {
            console.log(`[ReamaCubism] Texture ${modelTextureNumber} loaded: ${textureInfo.width}x${textureInfo.height}`);
            this.getRenderer().bindTexture(modelTextureNumber, textureInfo.id);
            this._textureCount++;
            if (this._textureCount >= textureCount) {
              console.log(`[ReamaCubism] All textures loaded. Model setup complete!`);
              this._state = 23 /* CompleteSetup */;
            }
          };
          this._subdelegate.getTextureManager().createTextureFromPngFile(texturePath, usePremultiply, onLoad);
          this.getRenderer().setIsPremultipliedAlpha(usePremultiply);
        }
        this._state = 22 /* WaitLoadTexture */;
      }
    }
    /**
     * レンダラを再構築する
     */
    reloadRenderer() {
      this.deleteRenderer();
      this.createRenderer(
        this._subdelegate.getCanvas().width,
        this._subdelegate.getCanvas().height
      );
      this.setupTextures();
    }
    /**
     * 更新
     */
    update() {
      if (this._state != 23 /* CompleteSetup */) return;
      const deltaTimeSeconds = LAppPal.getDeltaTime();
      this._userTimeSeconds += deltaTimeSeconds;
      this._model.loadParameters();
      this._motionUpdated = false;
      if (this._motionManager.isFinished()) {
        this.startRandomMotion(
          MotionGroupIdle,
          PriorityIdle
        );
      } else {
        this._motionUpdated = this._motionManager.updateMotion(
          this._model,
          deltaTimeSeconds
        );
      }
      this._model.saveParameters();
      this._updateScheduler.onLateUpdate(this._model, deltaTimeSeconds);
      this._model.update();
    }
    /**
     * 引数で指定したモーションの再生を開始する
     * @param group モーショングループ名
     * @param no グループ内の番号
     * @param priority 優先度
     * @param onFinishedMotionHandler モーション再生終了時に呼び出されるコールバック関数
     * @return 開始したモーションの識別番号を返す。個別のモーションが終了したか否かを判定するisFinished()の引数で使用する。開始できない時は[-1]
     */
    startMotion(group, no, priority, onFinishedMotionHandler, onBeganMotionHandler) {
      if (priority == PriorityForce) {
        this._motionManager.setReservePriority(priority);
      } else if (!this._motionManager.reserveMotion(priority)) {
        if (this._debugMode) {
          LAppPal.printMessage("[APP]can't start motion.");
        }
        return InvalidMotionQueueEntryHandleValue;
      }
      const motionFileName = this._modelSetting.getMotionFileName(group, no);
      const name = `${group}_${no}`;
      let motion = this._motions.get(name);
      let autoDelete = false;
      if (motion == null) {
        fetch(`${this._modelHomeDir}${motionFileName}`).then((response) => {
          if (response.ok) {
            return response.arrayBuffer();
          } else if (response.status >= 400) {
            CubismLogError(
              `Failed to load file ${this._modelHomeDir}${motionFileName}`
            );
            return new ArrayBuffer(0);
          }
        }).then((arrayBuffer) => {
          motion = this.loadMotion(
            arrayBuffer,
            arrayBuffer.byteLength,
            null,
            onFinishedMotionHandler,
            onBeganMotionHandler,
            this._modelSetting,
            group,
            no,
            this._motionConsistency
          );
        });
        if (motion) {
          motion.setEffectIds(this._eyeBlinkIds, this._lipSyncIds);
          autoDelete = true;
        } else {
          CubismLogError("Can't start motion {0} .", motionFileName);
          this._motionManager.setReservePriority(PriorityNone);
          return InvalidMotionQueueEntryHandleValue;
        }
      } else {
        motion.setBeganMotionHandler(onBeganMotionHandler);
        motion.setFinishedMotionHandler(onFinishedMotionHandler);
      }
      const voice = this._modelSetting.getMotionSoundFileName(group, no);
      if (voice.localeCompare("") != 0) {
        let path = voice;
        path = this._modelHomeDir + path;
        this._wavFileHandler.start(path);
      }
      if (this._debugMode) {
        LAppPal.printMessage(`[APP]start motion: [${group}_${no}]`);
      }
      return this._motionManager.startMotionPriority(
        motion,
        autoDelete,
        priority
      );
    }
    /**
     * ランダムに選ばれたモーションの再生を開始する。
     * @param group モーショングループ名
     * @param priority 優先度
     * @param onFinishedMotionHandler モーション再生終了時に呼び出されるコールバック関数
     * @return 開始したモーションの識別番号を返す。個別のモーションが終了したか否かを判定するisFinished()の引数で使用する。開始できない時は[-1]
     */
    startRandomMotion(group, priority, onFinishedMotionHandler, onBeganMotionHandler) {
      if (this._modelSetting.getMotionCount(group) == 0) {
        return InvalidMotionQueueEntryHandleValue;
      }
      const no = Math.floor(
        Math.random() * this._modelSetting.getMotionCount(group)
      );
      return this.startMotion(
        group,
        no,
        priority,
        onFinishedMotionHandler,
        onBeganMotionHandler
      );
    }
    /**
     * 引数で指定した表情モーションをセットする
     *
     * @param expressionId 表情モーションのID
     */
    setExpression(expressionId) {
      const motion = this._expressions.get(expressionId);
      if (this._debugMode) {
        LAppPal.printMessage(`[APP]expression: [${expressionId}]`);
      }
      if (motion != null) {
        this._expressionManager.startMotion(motion, false);
      } else {
        if (this._debugMode) {
          LAppPal.printMessage(`[APP]expression[${expressionId}] is null`);
        }
      }
    }
    /**
     * ランダムに選ばれた表情モーションをセットする
     */
    setRandomExpression() {
      if (this._expressions.size == 0) {
        return;
      }
      const no = Math.floor(Math.random() * this._expressions.size);
      for (let i = 0; i < this._expressions.size; i++) {
        if (i == no) {
          const expressionsArray = [...this._expressions.entries()];
          const name = expressionsArray[i][0];
          this.setExpression(name);
          return;
        }
      }
    }
    /**
     * イベントの発火を受け取る
     */
    motionEventFired(eventValue) {
      CubismLogInfo("{0} is fired on LAppModel!!", eventValue);
    }
    /**
     * 当たり判定テスト
     * 指定ＩＤの頂点リストから矩形を計算し、座標をが矩形範囲内か判定する。
     *
     * @param hitArenaName  当たり判定をテストする対象のID
     * @param x             判定を行うX座標
     * @param y             判定を行うY座標
     */
    hitTest(hitArenaName, x, y) {
      if (this._opacity < 1) {
        return false;
      }
      const count = this._modelSetting.getHitAreasCount();
      for (let i = 0; i < count; i++) {
        if (this._modelSetting.getHitAreaName(i) == hitArenaName) {
          const drawId = this._modelSetting.getHitAreaId(i);
          return this.isHit(drawId, x, y);
        }
      }
      return false;
    }
    /**
     * モーションデータをグループ名から一括でロードする。
     * モーションデータの名前は内部でModelSettingから取得する。
     *
     * @param group モーションデータのグループ名
     */
    preLoadMotionGroup(group) {
      for (let i = 0; i < this._modelSetting.getMotionCount(group); i++) {
        const motionFileName = this._modelSetting.getMotionFileName(group, i);
        const name = `${group}_${i}`;
        if (this._debugMode) {
          LAppPal.printMessage(
            `[APP]load motion: ${motionFileName} => [${name}]`
          );
        }
        fetch(`${this._modelHomeDir}${motionFileName}`).then((response) => {
          if (response.ok) {
            return response.arrayBuffer();
          } else if (response.status >= 400) {
            CubismLogError(
              `Failed to load file ${this._modelHomeDir}${motionFileName}`
            );
            return new ArrayBuffer(0);
          }
        }).then((arrayBuffer) => {
          const tmpMotion = this.loadMotion(
            arrayBuffer,
            arrayBuffer.byteLength,
            name,
            null,
            null,
            this._modelSetting,
            group,
            i,
            this._motionConsistency
          );
          if (tmpMotion != null) {
            tmpMotion.setEffectIds(this._eyeBlinkIds, this._lipSyncIds);
            if (this._motions.get(name) != null) {
              ACubismMotion.delete(this._motions.get(name));
            }
            this._motions.set(name, tmpMotion);
            this._motionCount++;
          } else {
            this._allMotionCount--;
          }
          if (this._motionCount >= this._allMotionCount) {
            this._state = 21 /* LoadTexture */;
            this._motionManager.stopAllMotions();
            this._updating = false;
            this._initialized = true;
            this.createRenderer(
              this._subdelegate.getCanvas().width,
              this._subdelegate.getCanvas().height
            );
            this.setupTextures();
            this.getRenderer().startUp(
              this._subdelegate.getGlManager().getGl()
            );
            this.getRenderer().loadShaders(ShaderPath);
          }
        });
      }
    }
    /**
     * すべてのモーションデータを解放する。
     */
    releaseMotions() {
      this._motions.clear();
    }
    /**
     * 全ての表情データを解放する。
     */
    releaseExpressions() {
      this._expressions.clear();
    }
    /**
     * モデルを描画する処理。モデルを描画する空間のView-Projection行列を渡す。
     */
    doDraw() {
      if (this._model == null) return;
      const canvas = this._subdelegate.getCanvas();
      const viewport = [0, 0, canvas.width, canvas.height];
      this.getRenderer().setRenderState(
        this._subdelegate.getFrameBuffer(),
        viewport
      );
      this.getRenderer().drawModel(ShaderPath);
    }
    /**
     * モデルを描画する処理。モデルを描画する空間のView-Projection行列を渡す。
     */
    draw(matrix) {
      if (this._model == null) {
        return;
      }
      if (this._state == 23 /* CompleteSetup */) {
        matrix.multiplyByMatrix(this._modelMatrix);
        this.getRenderer().setMvpMatrix(matrix);
        this.doDraw();
      }
    }
    async hasMocConsistencyFromFile() {
      CSM_ASSERT(this._modelSetting.getModelFileName().localeCompare(``));
      if (this._modelSetting.getModelFileName() != "") {
        const modelFileName = this._modelSetting.getModelFileName();
        const response = await fetch(`${this._modelHomeDir}${modelFileName}`);
        const arrayBuffer = await response.arrayBuffer();
        this._consistency = CubismMoc.hasMocConsistency(arrayBuffer);
        if (!this._consistency) {
          CubismLogInfo("Inconsistent MOC3.");
        } else {
          CubismLogInfo("Consistent MOC3.");
        }
        return this._consistency;
      } else {
        LAppPal.printMessage("Model data does not exist.");
      }
    }
    setSubdelegate(subdelegate) {
      this._subdelegate = subdelegate;
    }
    /**
     * デストラクタに相当する処理のオーバーライド
     */
    release() {
      if (this._look) {
        CubismLook.delete(this._look);
        this._look = null;
      }
      if (this._updateScheduler) {
        this._updateScheduler.release();
      }
      super.release();
    }
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._modelSetting = null;
      this._modelHomeDir = null;
      this._userTimeSeconds = 0;
      this._eyeBlinkIds = new Array();
      this._lipSyncIds = new Array();
      this._motions = /* @__PURE__ */ new Map();
      this._expressions = /* @__PURE__ */ new Map();
      this._hitArea = new Array();
      this._userArea = new Array();
      this._idParamAngleX = CubismFramework.getIdManager().getId(
        CubismDefaultParameterId.ParamAngleX
      );
      this._idParamAngleY = CubismFramework.getIdManager().getId(
        CubismDefaultParameterId.ParamAngleY
      );
      this._idParamAngleZ = CubismFramework.getIdManager().getId(
        CubismDefaultParameterId.ParamAngleZ
      );
      this._idParamBodyAngleX = CubismFramework.getIdManager().getId(
        CubismDefaultParameterId.ParamBodyAngleX
      );
      if (MOCConsistencyValidationEnable) {
        this._mocConsistency = true;
      }
      if (MotionConsistencyValidationEnable) {
        this._motionConsistency = true;
      }
      this._state = 0 /* LoadAssets */;
      this._expressionCount = 0;
      this._textureCount = 0;
      this._motionCount = 0;
      this._allMotionCount = 0;
      this._wavFileHandler = new LAppWavFileHandler();
      this._consistency = false;
      this._look = null;
      this._updateScheduler = new CubismUpdateScheduler();
      this._motionUpdated = false;
    }
    // MOC3整合性チェック管理用
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lapplive2dmanager.ts
  var LAppLive2DManager = class {
    /**
     * コンストラクタ
     */
    constructor() {
      // モーション再生開始のコールバック関数
      this.beganMotion = (self) => {
        LAppPal.printMessage("Motion Began:");
        console.log(self);
      };
      // モーション再生終了のコールバック関数
      this.finishedMotion = (self) => {
        LAppPal.printMessage("Motion Finished:");
        console.log(self);
      };
      this._subdelegate = null;
      this._viewMatrix = new CubismMatrix44();
      this._models = new Array();
      this._sceneIndex = 0;
      this._parameterDriver = null;
    }
    /**
     * 現在のシーンで保持しているすべてのモデルを解放する
     */
    releaseAllModel() {
      this._models.length = 0;
    }
    setOffscreenSize(width, height) {
      for (let i = 0; i < this._models.length; i++) {
        const model = this._models[i];
        model == null ? void 0 : model.setRenderTargetSize(width, height);
      }
    }
    /**
     * 画面をドラッグした時の処理
     *
     * @param x 画面のX座標
     * @param y 画面のY座標
     */
    onDrag(x, y) {
      const model = this._models[0];
      if (model) {
        model.setDragging(x, y);
      }
    }
    /**
     * 画面をタップした時の処理
     *
     * @param x 画面のX座標
     * @param y 画面のY座標
     */
    onTap(x, y) {
      if (DebugLogEnable) {
        LAppPal.printMessage(
          `[APP]tap point: {x: ${x.toFixed(2)} y: ${y.toFixed(2)}}`
        );
      }
      const model = this._models[0];
      if (model.hitTest(HitAreaNameHead, x, y)) {
        if (DebugLogEnable) {
          LAppPal.printMessage(`[APP]hit area: [${HitAreaNameHead}]`);
        }
        model.setRandomExpression();
      } else if (model.hitTest(HitAreaNameBody, x, y)) {
        if (DebugLogEnable) {
          LAppPal.printMessage(`[APP]hit area: [${HitAreaNameBody}]`);
        }
        model.startRandomMotion(
          MotionGroupTapBody,
          PriorityNormal,
          this.finishedMotion,
          this.beganMotion
        );
      }
    }
    /**
     * 画面を更新するときの処理
     * モデルの更新処理及び描画処理を行う
     */
    onUpdate() {
      const gl = this._subdelegate.getGl();
      CubismWebGLOffscreenManager.getInstance().beginFrameProcess(gl);
      const { width, height } = this._subdelegate.getCanvas();
      const projection = new CubismMatrix44();
      const model = this._models[0];
      if (!model) {
        CubismWebGLOffscreenManager.getInstance().endFrameProcess(gl);
        CubismWebGLOffscreenManager.getInstance().releaseStaleRenderTextures(gl);
        return;
      }
      if (model.getModel()) {
        if (model.getModel().getCanvasWidth() > 1 && width < height) {
          model.getModelMatrix().setWidth(2);
          projection.scale(1, width / height);
        } else {
          projection.scale(height / width, 1);
        }
        if (this._viewMatrix != null) {
          projection.multiplyByMatrix(this._viewMatrix);
        }
      }
      model.update();
      if (this._parameterDriver) {
        this._parameterDriver(model);
        // Re-compute vertex positions after our parameter overrides.
        // Without this, the Cubism core already finalized geometry inside
        // model.update() above, making our setParameterValueById calls invisible.
        if (model.getModel()) {
          model.getModel().update();
        }
      }
      if (width < height) {
        // Mobile: slightly smaller and shifted lower
        projection.scaleRelative(3.3, 3.3);
        projection.translateY(-2.1);
      } else {
        // Desktop
        projection.scaleRelative(4.1, 4.1);
        projection.translateY(-2.75);
      }
      model.draw(projection);
      CubismWebGLOffscreenManager.getInstance().endFrameProcess(gl);
      CubismWebGLOffscreenManager.getInstance().releaseStaleRenderTextures(gl);
    }
    /**
     * 次のシーンに切りかえる
     * サンプルアプリケーションではモデルセットの切り替えを行う。
     */
    nextScene() {
      const no = (this._sceneIndex + 1) % ModelDirSize;
      this.changeScene(no);
    }
    /**
     * シーンを切り替える
     * サンプルアプリケーションではモデルセットの切り替えを行う。
     * @param index
     */
    changeScene(index) {
      this._sceneIndex = index;
      if (DebugLogEnable) {
        LAppPal.printMessage(`[APP]model index: ${this._sceneIndex}`);
      }
      const model = ModelDir[index];
      const modelPath = ResourcesPath + model + "/";
      let modelJsonName = `${ModelDir[index]}.model3.json`;
      if (ModelJsonFileName) {
        modelJsonName = ModelJsonFileName;
      }
      this.releaseAllModel();
      const instance = new LAppModel();
      instance.setSubdelegate(this._subdelegate);
      instance.loadAssets(modelPath, modelJsonName);
      this._models.push(instance);
    }
    setViewMatrix(m) {
      for (let i = 0; i < 16; i++) {
        this._viewMatrix.getArray()[i] = m.getArray()[i];
      }
    }
    /**
     * モデルの追加
     */
    addModel(sceneIndex = 0) {
      this._sceneIndex = sceneIndex;
      this.changeScene(this._sceneIndex);
    }
    getMainModel() {
      var _a;
      return (_a = this._models[0]) != null ? _a : null;
    }
    setParameterDriver(driver) {
      this._parameterDriver = driver;
    }
    /**
     * 解放する。
     */
    release() {
    }
    /**
     * 初期化する。
     * @param subdelegate
     */
    initialize(subdelegate) {
      this._subdelegate = subdelegate;
      this.changeScene(this._sceneIndex);
    }
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lapptexturemanager.ts
  var LAppTextureManager = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._textures = new Array();
    }
    /**
     * 解放する。
     */
    release() {
      for (let i = 0; i < this._textures.length; i++) {
        this._glManager.getGl().deleteTexture(this._textures[i].id);
      }
      this._textures = null;
    }
    /**
     * 画像読み込み
     *
     * @param fileName 読み込む画像ファイルパス名
     * @param usePremultiply Premult処理を有効にするか
     * @return 画像情報、読み込み失敗時はnullを返す
     */
    createTextureFromPngFile(fileName, usePremultiply, callback) {
      for (let i = 0; i < this._textures.length; i++) {
        if (this._textures[i].fileName == fileName && this._textures[i].usePremultply == usePremultiply) {
          callback(this._textures[i]);
          return;
        }
      }
      cachedLoadImage(fileName).then((img) => {
        if (!this._glManager || !this._glManager.getGl()) {
          return;
        }
        const tex = this._glManager.getGl().createTexture();
        this._glManager.getGl().bindTexture(this._glManager.getGl().TEXTURE_2D, tex);
        this._glManager.getGl().texParameteri(
          this._glManager.getGl().TEXTURE_2D,
          this._glManager.getGl().TEXTURE_MIN_FILTER,
          this._glManager.getGl().LINEAR_MIPMAP_LINEAR
        );
        this._glManager.getGl().texParameteri(
          this._glManager.getGl().TEXTURE_2D,
          this._glManager.getGl().TEXTURE_MAG_FILTER,
          this._glManager.getGl().LINEAR
        );
        if (usePremultiply) {
          this._glManager.getGl().pixelStorei(
            this._glManager.getGl().UNPACK_PREMULTIPLY_ALPHA_WEBGL,
            1
          );
        }
        const gl = this._glManager.getGl();
        const maxTextureSize = gl.getParameter(gl.MAX_TEXTURE_SIZE);
        if (img.width > maxTextureSize || img.height > maxTextureSize) {
          console.warn(`[ReamaCubism] Texture ${fileName} (${img.width}x${img.height}) exceeds MAX_TEXTURE_SIZE (${maxTextureSize}). Downscaling...`);
          const scale = maxTextureSize / Math.max(img.width, img.height);
          const offscreen = document.createElement("canvas");
          offscreen.width = Math.floor(img.width * scale);
          offscreen.height = Math.floor(img.height * scale);
          const ctx = offscreen.getContext("2d");
          if (ctx) {
            ctx.drawImage(img, 0, 0, offscreen.width, offscreen.height);
            gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, offscreen);
          }
        } else {
          gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
        }
        this._glManager.getGl().generateMipmap(this._glManager.getGl().TEXTURE_2D);
        this._glManager.getGl().bindTexture(this._glManager.getGl().TEXTURE_2D, null);
        const textureInfo = new TextureInfo();
        if (textureInfo != null) {
          textureInfo.fileName = fileName;
          textureInfo.width = img.width;
          textureInfo.height = img.height;
          textureInfo.id = tex;
          textureInfo.img = img;
          textureInfo.usePremultply = usePremultiply;
          if (this._textures != null) {
            this._textures.push(textureInfo);
          }
        }
        callback(textureInfo);
      }).catch((e) => {
        console.error(`[ReamaCubism] Failed to load texture image: ${fileName}`, e);
      });
    }
    /**
     * 画像の解放
     *
     * 配列に存在する画像全てを解放する。
     */
    releaseTextures() {
      for (let i = 0; i < this._textures.length; i++) {
        this._glManager.getGl().deleteTexture(this._textures[i].id);
        this._textures[i] = null;
      }
      this._textures.length = 0;
    }
    /**
     * 画像の解放
     *
     * 指定したテクスチャの画像を解放する。
     * @param texture 解放するテクスチャ
     */
    releaseTextureByTexture(texture) {
      for (let i = 0; i < this._textures.length; i++) {
        if (this._textures[i].id != texture) {
          continue;
        }
        this._glManager.getGl().deleteTexture(this._textures[i].id);
        this._textures[i] = null;
        this._textures.splice(i, 1);
        break;
      }
    }
    /**
     * 画像の解放
     *
     * 指定した名前の画像を解放する。
     * @param fileName 解放する画像ファイルパス名
     */
    releaseTextureByFilePath(fileName) {
      for (let i = 0; i < this._textures.length; i++) {
        if (this._textures[i].fileName == fileName) {
          this._glManager.getGl().deleteTexture(this._textures[i].id);
          this._textures[i] = null;
          this._textures.splice(i, 1);
          break;
        }
      }
    }
    /**
     * setter
     * @param glManager
     */
    setGlManager(glManager) {
      this._glManager = glManager;
    }
  };
  var TextureInfo = class {
    constructor() {
      // 画像
      this.id = null;
      // テクスチャ
      this.width = 0;
      // 横幅
      this.height = 0;
    }
    // ファイル名
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Framework/src/math/cubismviewmatrix.ts
  var CubismViewMatrix = class extends CubismMatrix44 {
    /**
     * コンストラクタ
     */
    constructor() {
      super();
      this._screenLeft = 0;
      this._screenRight = 0;
      this._screenTop = 0;
      this._screenBottom = 0;
      this._maxLeft = 0;
      this._maxRight = 0;
      this._maxTop = 0;
      this._maxBottom = 0;
      this._maxScale = 0;
      this._minScale = 0;
    }
    /**
     * 移動を調整
     *
     * @param x X軸の移動量
     * @param y Y軸の移動量
     */
    adjustTranslate(x, y) {
      if (this._tr[0] * this._maxLeft + (this._tr[12] + x) > this._screenLeft) {
        x = this._screenLeft - this._tr[0] * this._maxLeft - this._tr[12];
      }
      if (this._tr[0] * this._maxRight + (this._tr[12] + x) < this._screenRight) {
        x = this._screenRight - this._tr[0] * this._maxRight - this._tr[12];
      }
      if (this._tr[5] * this._maxTop + (this._tr[13] + y) < this._screenTop) {
        y = this._screenTop - this._tr[5] * this._maxTop - this._tr[13];
      }
      if (this._tr[5] * this._maxBottom + (this._tr[13] + y) > this._screenBottom) {
        y = this._screenBottom - this._tr[5] * this._maxBottom - this._tr[13];
      }
      const tr1 = new Float32Array([
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        x,
        y,
        0,
        1
      ]);
      CubismMatrix44.multiply(tr1, this._tr, this._tr);
    }
    /**
     * 拡大率を調整
     *
     * @param cx 拡大を行うX軸の中心位置
     * @param cy 拡大を行うY軸の中心位置
     * @param scale 拡大率
     */
    adjustScale(cx, cy, scale) {
      const maxScale = this.getMaxScale();
      const minScale = this.getMinScale();
      const targetScale = scale * this._tr[0];
      if (targetScale < minScale) {
        if (this._tr[0] > 0) {
          scale = minScale / this._tr[0];
        }
      } else if (targetScale > maxScale) {
        if (this._tr[0] > 0) {
          scale = maxScale / this._tr[0];
        }
      }
      const tr1 = new Float32Array([
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        cx,
        cy,
        0,
        1
      ]);
      const tr2 = new Float32Array([
        scale,
        0,
        0,
        0,
        0,
        scale,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1
      ]);
      const tr3 = new Float32Array([
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        0,
        0,
        0,
        1,
        0,
        -cx,
        -cy,
        0,
        1
      ]);
      CubismMatrix44.multiply(tr3, this._tr, this._tr);
      CubismMatrix44.multiply(tr2, this._tr, this._tr);
      CubismMatrix44.multiply(tr1, this._tr, this._tr);
    }
    /**
     * デバイスに対応する論理座養生の範囲の設定
     *
     * @param left      左辺のX軸の位置
     * @param right     右辺のX軸の位置
     * @param bottom    下辺のY軸の位置
     * @param top       上辺のY軸の位置
     */
    setScreenRect(left, right, bottom, top) {
      this._screenLeft = left;
      this._screenRight = right;
      this._screenBottom = bottom;
      this._screenTop = top;
    }
    /**
     * デバイスに対応する論理座標上の移動可能範囲の設定
     * @param left      左辺のX軸の位置
     * @param right     右辺のX軸の位置
     * @param bottom    下辺のY軸の位置
     * @param top       上辺のY軸の位置
     */
    setMaxScreenRect(left, right, bottom, top) {
      this._maxLeft = left;
      this._maxRight = right;
      this._maxTop = top;
      this._maxBottom = bottom;
    }
    /**
     * 最大拡大率の設定
     * @param maxScale 最大拡大率
     */
    setMaxScale(maxScale) {
      this._maxScale = maxScale;
    }
    /**
     * 最小拡大率の設定
     * @param minScale 最小拡大率
     */
    setMinScale(minScale) {
      this._minScale = minScale;
    }
    /**
     * 最大拡大率の取得
     * @return 最大拡大率
     */
    getMaxScale() {
      return this._maxScale;
    }
    /**
     * 最小拡大率の取得
     * @return 最小拡大率
     */
    getMinScale() {
      return this._minScale;
    }
    /**
     * 拡大率が最大になっているかを確認する
     *
     * @return true 拡大率は最大
     * @return false 拡大率は最大ではない
     */
    isMaxScale() {
      return this.getScaleX() >= this._maxScale;
    }
    /**
     * 拡大率が最小になっているかを確認する
     *
     * @return true 拡大率は最小
     * @return false 拡大率は最小ではない
     */
    isMinScale() {
      return this.getScaleX() <= this._minScale;
    }
    /**
     * デバイスに対応する論理座標の左辺のＸ軸位置を取得する
     * @return デバイスに対応する論理座標の左辺のX軸位置
     */
    getScreenLeft() {
      return this._screenLeft;
    }
    /**
     * デバイスに対応する論理座標の右辺のＸ軸位置を取得する
     * @return デバイスに対応する論理座標の右辺のX軸位置
     */
    getScreenRight() {
      return this._screenRight;
    }
    /**
     * デバイスに対応する論理座標の下辺のY軸位置を取得する
     * @return デバイスに対応する論理座標の下辺のY軸位置
     */
    getScreenBottom() {
      return this._screenBottom;
    }
    /**
     * デバイスに対応する論理座標の上辺のY軸位置を取得する
     * @return デバイスに対応する論理座標の上辺のY軸位置
     */
    getScreenTop() {
      return this._screenTop;
    }
    /**
     * 左辺のX軸位置の最大値の取得
     * @return 左辺のX軸位置の最大値
     */
    getMaxLeft() {
      return this._maxLeft;
    }
    /**
     * 右辺のX軸位置の最大値の取得
     * @return 右辺のX軸位置の最大値
     */
    getMaxRight() {
      return this._maxRight;
    }
    /**
     * 下辺のY軸位置の最大値の取得
     * @return 下辺のY軸位置の最大値
     */
    getMaxBottom() {
      return this._maxBottom;
    }
    /**
     * 上辺のY軸位置の最大値の取得
     * @return 上辺のY軸位置の最大値
     */
    getMaxTop() {
      return this._maxTop;
    }
    // 拡大率の最小値
  };
  var Live2DCubismFramework50;
  ((Live2DCubismFramework51) => {
    Live2DCubismFramework51.CubismViewMatrix = CubismViewMatrix;
  })(Live2DCubismFramework50 || (Live2DCubismFramework50 = {}));

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappsprite.ts
  var LAppSprite = class {
    /**
     * コンストラクタ
     * @param x            x座標
     * @param y            y座標
     * @param width        横幅
     * @param height       高さ
     * @param textureId    テクスチャ
     */
    constructor(x, y, width, height, textureId) {
      this._rect = new Rect();
      this._rect.left = x - width * 0.5;
      this._rect.right = x + width * 0.5;
      this._rect.up = y + height * 0.5;
      this._rect.down = y - height * 0.5;
      this._texture = textureId;
      this._vertexBuffer = null;
      this._uvBuffer = null;
      this._indexBuffer = null;
      this._positionLocation = null;
      this._uvLocation = null;
      this._textureLocation = null;
      this._positionArray = null;
      this._uvArray = null;
      this._indexArray = null;
      this._firstDraw = true;
    }
    /**
     * 解放する。
     */
    release() {
      this._rect = null;
      const gl = this._subdelegate.getGlManager().getGl();
      gl.deleteTexture(this._texture);
      this._texture = null;
      gl.deleteBuffer(this._uvBuffer);
      this._uvBuffer = null;
      gl.deleteBuffer(this._vertexBuffer);
      this._vertexBuffer = null;
      gl.deleteBuffer(this._indexBuffer);
      this._indexBuffer = null;
    }
    /**
     * テクスチャを返す
     */
    getTexture() {
      return this._texture;
    }
    /**
     * 描画する。
     * @param programId シェーダープログラム
     * @param canvas 描画するキャンパス情報
     */
    render(programId) {
      if (this._texture == null) {
        return;
      }
      const gl = this._subdelegate.getGlManager().getGl();
      if (this._firstDraw) {
        this._positionLocation = gl.getAttribLocation(programId, "position");
        gl.enableVertexAttribArray(this._positionLocation);
        this._uvLocation = gl.getAttribLocation(programId, "uv");
        gl.enableVertexAttribArray(this._uvLocation);
        this._textureLocation = gl.getUniformLocation(programId, "texture");
        gl.uniform1i(this._textureLocation, 0);
        {
          this._uvArray = new Float32Array([
            1,
            0,
            0,
            0,
            0,
            1,
            1,
            1
          ]);
          this._uvBuffer = gl.createBuffer();
        }
        {
          const maxWidth = this._subdelegate.getCanvas().width;
          const maxHeight = this._subdelegate.getCanvas().height;
          this._positionArray = new Float32Array([
            (this._rect.right - maxWidth * 0.5) / (maxWidth * 0.5),
            (this._rect.up - maxHeight * 0.5) / (maxHeight * 0.5),
            (this._rect.left - maxWidth * 0.5) / (maxWidth * 0.5),
            (this._rect.up - maxHeight * 0.5) / (maxHeight * 0.5),
            (this._rect.left - maxWidth * 0.5) / (maxWidth * 0.5),
            (this._rect.down - maxHeight * 0.5) / (maxHeight * 0.5),
            (this._rect.right - maxWidth * 0.5) / (maxWidth * 0.5),
            (this._rect.down - maxHeight * 0.5) / (maxHeight * 0.5)
          ]);
          this._vertexBuffer = gl.createBuffer();
        }
        {
          this._indexArray = new Uint16Array([0, 1, 2, 3, 2, 0]);
          this._indexBuffer = gl.createBuffer();
        }
        this._firstDraw = false;
      }
      gl.bindBuffer(gl.ARRAY_BUFFER, this._uvBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, this._uvArray, gl.STATIC_DRAW);
      gl.vertexAttribPointer(this._uvLocation, 2, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, this._vertexBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, this._positionArray, gl.STATIC_DRAW);
      gl.vertexAttribPointer(this._positionLocation, 2, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this._indexBuffer);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, this._indexArray, gl.DYNAMIC_DRAW);
      gl.bindTexture(gl.TEXTURE_2D, this._texture);
      gl.drawElements(
        gl.TRIANGLES,
        this._indexArray.length,
        gl.UNSIGNED_SHORT,
        0
      );
    }
    /**
     * 当たり判定
     * @param pointX x座標
     * @param pointY y座標
     */
    isHit(pointX, pointY) {
      const { height } = this._subdelegate.getCanvas();
      const y = height - pointY;
      return pointX >= this._rect.left && pointX <= this._rect.right && y <= this._rect.up && y >= this._rect.down;
    }
    /**
     * setter
     * @param subdelegate
     */
    setSubdelegate(subdelegate) {
      this._subdelegate = subdelegate;
    }
  };
  var Rect = class {
    // 下辺
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/touchmanager.ts
  var TouchManager = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._startX = 0;
      this._startY = 0;
      this._lastX = 0;
      this._lastY = 0;
      this._lastX1 = 0;
      this._lastY1 = 0;
      this._lastX2 = 0;
      this._lastY2 = 0;
      this._lastTouchDistance = 0;
      this._deltaX = 0;
      this._deltaY = 0;
      this._scale = 1;
      this._touchSingle = false;
      this._flipAvailable = false;
    }
    getCenterX() {
      return this._lastX;
    }
    getCenterY() {
      return this._lastY;
    }
    getDeltaX() {
      return this._deltaX;
    }
    getDeltaY() {
      return this._deltaY;
    }
    getStartX() {
      return this._startX;
    }
    getStartY() {
      return this._startY;
    }
    getScale() {
      return this._scale;
    }
    getX() {
      return this._lastX;
    }
    getY() {
      return this._lastY;
    }
    getX1() {
      return this._lastX1;
    }
    getY1() {
      return this._lastY1;
    }
    getX2() {
      return this._lastX2;
    }
    getY2() {
      return this._lastY2;
    }
    isSingleTouch() {
      return this._touchSingle;
    }
    isFlickAvailable() {
      return this._flipAvailable;
    }
    disableFlick() {
      this._flipAvailable = false;
    }
    /**
     * タッチ開始時イベント
     * @param deviceX タッチした画面のxの値
     * @param deviceY タッチした画面のyの値
     */
    touchesBegan(deviceX, deviceY) {
      this._lastX = deviceX;
      this._lastY = deviceY;
      this._startX = deviceX;
      this._startY = deviceY;
      this._lastTouchDistance = -1;
      this._flipAvailable = true;
      this._touchSingle = true;
    }
    /**
     * ドラッグ時のイベント
     * @param deviceX タッチした画面のxの値
     * @param deviceY タッチした画面のyの値
     */
    touchesMoved(deviceX, deviceY) {
      this._lastX = deviceX;
      this._lastY = deviceY;
      this._lastTouchDistance = -1;
      this._touchSingle = true;
    }
    /**
     * フリックの距離測定
     * @return フリック距離
     */
    getFlickDistance() {
      return this.calculateDistance(
        this._startX,
        this._startY,
        this._lastX,
        this._lastY
      );
    }
    /**
     * 点１から点２への距離を求める
     *
     * @param x1 １つ目のタッチした画面のxの値
     * @param y1 １つ目のタッチした画面のyの値
     * @param x2 ２つ目のタッチした画面のxの値
     * @param y2 ２つ目のタッチした画面のyの値
     */
    calculateDistance(x1, y1, x2, y2) {
      return Math.sqrt((x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2));
    }
    /**
     * ２つ目の値から、移動量を求める。
     * 違う方向の場合は移動量０。同じ方向の場合は、絶対値が小さい方の値を参照する。
     *
     * @param v1 １つ目の移動量
     * @param v2 ２つ目の移動量
     *
     * @return 小さい方の移動量
     */
    calculateMovingAmount(v1, v2) {
      if (v1 > 0 != v2 > 0) {
        return 0;
      }
      const sign2 = v1 > 0 ? 1 : -1;
      const absoluteValue1 = Math.abs(v1);
      const absoluteValue2 = Math.abs(v2);
      return sign2 * (absoluteValue1 < absoluteValue2 ? absoluteValue1 : absoluteValue2);
    }
    // フリップが有効かどうか
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappview.ts
  var LAppView = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._programId = null;
      this._back = null;
      this._gear = null;
      this._touchManager = new TouchManager();
      this._deviceToScreen = new CubismMatrix44();
      this._viewMatrix = new CubismViewMatrix();
    }
    /**
     * 初期化する。
     */
    initialize(subdelegate) {
      this._subdelegate = subdelegate;
      const { width, height } = subdelegate.getCanvas();
      const ratio = width / height;
      const left = -ratio;
      const right = ratio;
      const bottom = ViewLogicalLeft;
      const top = ViewLogicalRight;
      this._viewMatrix.setScreenRect(left, right, bottom, top);
      this._viewMatrix.scale(ViewScale, ViewScale);
      this._deviceToScreen.loadIdentity();
      if (width > height) {
        const screenW = Math.abs(right - left);
        this._deviceToScreen.scaleRelative(screenW / width, -screenW / width);
      } else {
        const screenH = Math.abs(top - bottom);
        this._deviceToScreen.scaleRelative(screenH / height, -screenH / height);
      }
      this._deviceToScreen.translateRelative(-width * 0.5, -height * 0.5);
      this._viewMatrix.setMaxScale(ViewMaxScale);
      this._viewMatrix.setMinScale(ViewMinScale);
      this._viewMatrix.setMaxScreenRect(
        ViewLogicalMaxLeft,
        ViewLogicalMaxRight,
        ViewLogicalMaxBottom,
        ViewLogicalMaxTop
      );
    }
    /**
     * 解放する
     */
    release() {
      this._viewMatrix = null;
      this._touchManager = null;
      this._deviceToScreen = null;
      this._gear.release();
      this._gear = null;
      this._back.release();
      this._back = null;
      this._subdelegate.getGlManager().getGl().deleteProgram(this._programId);
      this._programId = null;
    }
    /**
     * 描画する。
     */
    render() {
      this._subdelegate.getGlManager().getGl().useProgram(this._programId);
      if (this._back) {
        this._back.render(this._programId);
      }
      if (this._gear) {
        this._gear.render(this._programId);
      }
      this._subdelegate.getGlManager().getGl().flush();
      const lapplive2dmanager = this._subdelegate.getLive2DManager();
      if (lapplive2dmanager != null) {
        lapplive2dmanager.setViewMatrix(this._viewMatrix);
        lapplive2dmanager.onUpdate();
      }
    }
    /**
     * 画像の初期化を行う。
     */
    initializeSprite() {
      const width = this._subdelegate.getCanvas().width;
      const height = this._subdelegate.getCanvas().height;
      const textureManager = this._subdelegate.getTextureManager();
      const resourcesPath = ResourcesPath;
      if (BackImageName) {
        const initBackGroundTexture = (textureInfo) => {
          const x = width * 0.5;
          const y = height * 0.5;
          const fheight = height * 0.95;
          const ratio = fheight / textureInfo.height;
          const fwidth = textureInfo.width * ratio;
          this._back = new LAppSprite(x, y, fwidth, fheight, textureInfo.id);
          this._back.setSubdelegate(this._subdelegate);
        };
        textureManager.createTextureFromPngFile(
          resourcesPath + BackImageName,
          false,
          initBackGroundTexture
        );
      }
      if (GearImageName) {
        const initGearTexture = (textureInfo) => {
          const x = width - textureInfo.width * 0.5;
          const y = height - textureInfo.height * 0.5;
          const fwidth = textureInfo.width;
          const fheight = textureInfo.height;
          this._gear = new LAppSprite(x, y, fwidth, fheight, textureInfo.id);
          this._gear.setSubdelegate(this._subdelegate);
        };
        textureManager.createTextureFromPngFile(
          resourcesPath + GearImageName,
          false,
          initGearTexture
        );
      }
      if (this._programId == null) {
        this._programId = this._subdelegate.createShader();
      }
    }
    /**
     * タッチされた時に呼ばれる。
     *
     * @param pointX スクリーンX座標
     * @param pointY スクリーンY座標
     */
    onTouchesBegan(pointX, pointY) {
      this._touchManager.touchesBegan(
        pointX * window.devicePixelRatio,
        pointY * window.devicePixelRatio
      );
    }
    /**
     * タッチしているときにポインタが動いたら呼ばれる。
     *
     * @param pointX スクリーンX座標
     * @param pointY スクリーンY座標
     */
    onTouchesMoved(pointX, pointY) {
      const posX = pointX * window.devicePixelRatio;
      const posY = pointY * window.devicePixelRatio;
      const lapplive2dmanager = this._subdelegate.getLive2DManager();
      const viewX = this.transformViewX(this._touchManager.getX());
      const viewY = this.transformViewY(this._touchManager.getY());
      this._touchManager.touchesMoved(posX, posY);
      lapplive2dmanager.onDrag(viewX, viewY);
    }
    /**
     * タッチが終了したら呼ばれる。
     *
     * @param pointX スクリーンX座標
     * @param pointY スクリーンY座標
     */
    onTouchesEnded(pointX, pointY) {
      const posX = pointX * window.devicePixelRatio;
      const posY = pointY * window.devicePixelRatio;
      const lapplive2dmanager = this._subdelegate.getLive2DManager();
      lapplive2dmanager.onDrag(0, 0);
      const x = this.transformViewX(posX);
      const y = this.transformViewY(posY);
      if (DebugTouchLogEnable) {
        LAppPal.printMessage(`[APP]touchesEnded x: ${x} y: ${y}`);
      }
      lapplive2dmanager.onTap(x, y);
      if (this._gear && this._gear.isHit(posX, posY)) {
        lapplive2dmanager.nextScene();
      }
    }
    /**
     * X座標をView座標に変換する。
     *
     * @param deviceX デバイスX座標
     */
    transformViewX(deviceX) {
      const screenX = this._deviceToScreen.transformX(deviceX);
      return this._viewMatrix.invertTransformX(screenX);
    }
    /**
     * Y座標をView座標に変換する。
     *
     * @param deviceY デバイスY座標
     */
    transformViewY(deviceY) {
      const screenY = this._deviceToScreen.transformY(deviceY);
      return this._viewMatrix.invertTransformY(screenY);
    }
    /**
     * X座標をScreen座標に変換する。
     * @param deviceX デバイスX座標
     */
    transformScreenX(deviceX) {
      return this._deviceToScreen.transformX(deviceX);
    }
    /**
     * Y座標をScreen座標に変換する。
     *
     * @param deviceY デバイスY座標
     */
    transformScreenY(deviceY) {
      return this._deviceToScreen.transformY(deviceY);
    }
  };

  // CubismSdkForWeb-5-r.5/CubismSdkForWeb-5-r.5/Samples/TypeScript/Demo/src/lappsubdelegate.ts
  var LAppSubdelegate = class {
    /**
     * コンストラクタ
     */
    constructor() {
      this._canvas = null;
      this._glManager = new LAppGlManager();
      this._textureManager = new LAppTextureManager();
      this._live2dManager = new LAppLive2DManager();
      this._view = new LAppView();
      this._frameBuffer = null;
      this._captured = false;
    }
    /**
     * デストラクタ相当の処理
     */
    release() {
      this._resizeObserver.unobserve(this._canvas);
      this._resizeObserver.disconnect();
      this._resizeObserver = null;
      this._live2dManager.release();
      this._live2dManager = null;
      this._view.release();
      this._view = null;
      this._textureManager.release();
      this._textureManager = null;
      this._glManager.release();
      this._glManager = null;
    }
    /**
     * APPに必要な物を初期化する。
     */
    initialize(canvas) {
      if (!this._glManager.initialize(canvas)) {
        return false;
      }
      this._canvas = canvas;
      if (CanvasSize === "auto") {
        this.resizeCanvas();
      } else {
        canvas.width = CanvasSize.width;
        canvas.height = CanvasSize.height;
      }
      this._textureManager.setGlManager(this._glManager);
      const gl = this._glManager.getGl();
      if (!this._frameBuffer) {
        this._frameBuffer = gl.getParameter(gl.FRAMEBUFFER_BINDING);
      }
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
      this._view.initialize(this);
      this._live2dManager.setOffscreenSize(
        this._canvas.width,
        this._canvas.height
      );
      this._view.initializeSprite();
      this._live2dManager.initialize(this);
      this._resizeObserver = new ResizeObserver(
        (entries, observer) => this.resizeObserverCallback.call(this, entries, observer)
      );
      this._resizeObserver.observe(this._canvas);
      return true;
    }
    /**
     * Resize canvas and re-initialize view.
     */
    onResize() {
      this.resizeCanvas();
      this._view.initialize(this);
      this._live2dManager.setOffscreenSize(
        this._canvas.width,
        this._canvas.height
      );
    }
    resizeObserverCallback(entries, observer) {
      if (CanvasSize === "auto") {
        this._needResize = true;
      }
    }
    /**
     * ループ処理
     */
    update() {
      if (this._glManager.getGl().isContextLost()) {
        return;
      }
      if (this._needResize) {
        this.onResize();
        this._needResize = false;
      }
      const gl = this._glManager.getGl();
      gl.clearColor(0, 0, 0, 0);
      gl.enable(gl.DEPTH_TEST);
      gl.depthFunc(gl.LEQUAL);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.clearDepth(1);
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
      this._view.render();
    }
    /**
     * シェーダーを登録する。
     */
    createShader() {
      const gl = this._glManager.getGl();
      const vertexShaderId = gl.createShader(gl.VERTEX_SHADER);
      if (vertexShaderId == null) {
        LAppPal.printMessage("failed to create vertexShader");
        return null;
      }
      const vertexShader = "precision mediump float;attribute vec3 position;attribute vec2 uv;varying vec2 vuv;void main(void){   gl_Position = vec4(position, 1.0);   vuv = uv;}";
      gl.shaderSource(vertexShaderId, vertexShader);
      gl.compileShader(vertexShaderId);
      const fragmentShaderId = gl.createShader(gl.FRAGMENT_SHADER);
      if (fragmentShaderId == null) {
        LAppPal.printMessage("failed to create fragmentShader");
        return null;
      }
      const fragmentShader = "precision mediump float;varying vec2 vuv;uniform sampler2D texture;void main(void){   gl_FragColor = texture2D(texture, vuv);}";
      gl.shaderSource(fragmentShaderId, fragmentShader);
      gl.compileShader(fragmentShaderId);
      const programId = gl.createProgram();
      gl.attachShader(programId, vertexShaderId);
      gl.attachShader(programId, fragmentShaderId);
      gl.deleteShader(vertexShaderId);
      gl.deleteShader(fragmentShaderId);
      gl.linkProgram(programId);
      gl.useProgram(programId);
      return programId;
    }
    getTextureManager() {
      return this._textureManager;
    }
    getFrameBuffer() {
      return this._frameBuffer;
    }
    getCanvas() {
      return this._canvas;
    }
    getGlManager() {
      return this._glManager;
    }
    getGl() {
      return this._glManager.getGl();
    }
    getLive2DManager() {
      return this._live2dManager;
    }
    /**
     * Resize the canvas to fill the screen.
     */
    resizeCanvas() {
      this._canvas.width = this._canvas.clientWidth * window.devicePixelRatio;
      this._canvas.height = this._canvas.clientHeight * window.devicePixelRatio;
      const gl = this._glManager.getGl();
      gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    }
    /**
     * マウスダウン、タッチダウンしたときに呼ばれる。
     */
    onPointBegan(pageX, pageY) {
      if (!this._view) {
        LAppPal.printMessage("view notfound");
        return;
      }
      this._captured = true;
      const localX = pageX - this._canvas.offsetLeft;
      const localY = pageY - this._canvas.offsetTop;
      this._view.onTouchesBegan(localX, localY);
    }
    /**
     * マウスポインタが動いたら呼ばれる。
     */
    onPointMoved(pageX, pageY) {
      if (!this._captured) {
        return;
      }
      const localX = pageX - this._canvas.offsetLeft;
      const localY = pageY - this._canvas.offsetTop;
      this._view.onTouchesMoved(localX, localY);
    }
    /**
     * クリックが終了したら呼ばれる。
     */
    onPointEnded(pageX, pageY) {
      this._captured = false;
      if (!this._view) {
        LAppPal.printMessage("view notfound");
        return;
      }
      const localX = pageX - this._canvas.offsetLeft;
      const localY = pageY - this._canvas.offsetTop;
      this._view.onTouchesEnded(localX, localY);
    }
    /**
     * タッチがキャンセルされると呼ばれる。
     */
    onTouchCancel(pageX, pageY) {
      this._captured = false;
      if (!this._view) {
        LAppPal.printMessage("view notfound");
        return;
      }
      const localX = pageX - this._canvas.offsetLeft;
      const localY = pageY - this._canvas.offsetTop;
      this._view.onTouchesEnded(localX, localY);
    }
    isContextLost() {
      return this._glManager.getGl().isContextLost();
    }
  };

  // scripts/cubism-runtime-entry.ts
  var EMOTION_TARGETS = {
    NORMAL: {
      browY: 0,
      browForm: 0.5,
      browAngle: 0,
      eyeOpen: 1,
      eyeSmile: 0,
      mouthForm: 0,
      bodyAngleX: 0,
      bodyAngleY: 0,
      bodyAngleZ: 0,
      headAngleX: 0,
      headAngleY: 0,
      headAngleZ: 0,
      cheek: 0
    },
    SMILE: {
      browY: 0.4,
      browForm: 0.8,
      browAngle: 0.2,
      eyeOpen: 0.6,
      eyeSmile: 1,
      mouthForm: 1,
      bodyAngleX: 3,
      bodyAngleY: 0,
      bodyAngleZ: -2,
      headAngleX: 5,
      headAngleY: 3,
      headAngleZ: -3,
      cheek: 0.3
    },
    ANGRY: {
      browY: -0.6,
      browForm: -0.8,
      browAngle: -0.8,
      eyeOpen: 0.5,
      eyeSmile: 0,
      mouthForm: -0.8,
      bodyAngleX: -4,
      bodyAngleY: 0,
      bodyAngleZ: 0,
      headAngleX: -5,
      headAngleY: -5,
      headAngleZ: 0,
      cheek: 0
    },
    SAD: {
      browY: -0.5,
      browForm: -0.6,
      browAngle: 0.6,
      eyeOpen: 0.4,
      eyeSmile: 0,
      mouthForm: -0.5,
      bodyAngleX: -3,
      bodyAngleY: -3,
      bodyAngleZ: -3,
      headAngleX: -8,
      headAngleY: -7,
      headAngleZ: -5,
      cheek: 0
    },
    SURPRISED: {
      browY: 0.8,
      browForm: 0.5,
      browAngle: 0,
      eyeOpen: 1.25,
      eyeSmile: 0,
      mouthForm: -0.4,
      bodyAngleX: -2,
      bodyAngleY: 5,
      bodyAngleZ: 2,
      headAngleX: 2,
      headAngleY: 8,
      headAngleZ: 0,
      cheek: 0
    },
    BLUSH: {
      browY: 0.2,
      browForm: 0.4,
      browAngle: 0.3,
      eyeOpen: 0.6,
      eyeSmile: 0.7,
      mouthForm: 0.3,
      bodyAngleX: 8,
      bodyAngleY: -3,
      bodyAngleZ: -3,
      headAngleX: 8,
      headAngleY: -8,
      headAngleZ: -7,
      cheek: 1
    },
    WINK: {
      browY: 0.5,
      browForm: 0.8,
      browAngle: 0.2,
      eyeOpen: 1,
      eyeSmile: 0.7,
      mouthForm: 0.5,
      bodyAngleX: 2,
      bodyAngleY: 0,
      bodyAngleZ: -5,
      headAngleX: 5,
      headAngleY: 2,
      headAngleZ: -5,
      cheek: 0,
      isWink: true
    },
    DISGUST: {
      browY: -0.3,
      browForm: -0.9,
      browAngle: -0.5,
      eyeOpen: 0.4,
      eyeSmile: 0,
      mouthForm: -0.9,
      bodyAngleX: 5,
      bodyAngleY: 4,
      bodyAngleZ: 2,
      headAngleX: 5,
      headAngleY: -4,
      headAngleZ: 3,
      cheek: 0
    },
    SMUG: {
      browY: 0.3,
      browForm: 0.7,
      browAngle: 0.4,
      eyeOpen: 0.6,
      eyeSmile: 0.8,
      mouthForm: 0.6,
      bodyAngleX: 4,
      bodyAngleY: 0,
      bodyAngleZ: -2,
      headAngleX: 10,
      headAngleY: 5,
      headAngleZ: -2,
      cheek: 0
    },
    THINKING: {
      browY: -0.2,
      browForm: -0.3,
      browAngle: 0.2,
      eyeOpen: 0.8,
      eyeSmile: 0,
      mouthForm: -0.2,
      bodyAngleX: -2,
      bodyAngleY: 5,
      bodyAngleZ: 5,
      headAngleX: 5,
      headAngleY: -8,
      headAngleZ: 5,
      cheek: 0
    },
    PANIC: {
      browY: 0.6,
      browForm: -0.5,
      browAngle: -0.3,
      eyeOpen: 1.2,
      eyeSmile: 0,
      mouthForm: -0.5,
      bodyAngleX: -3,
      bodyAngleY: 0,
      bodyAngleZ: 0,
      headAngleX: -2,
      headAngleY: 0,
      headAngleZ: 0,
      cheek: 0.6
    },
    SLEEPING: {
      browY: -0.3,
      browForm: 0.2,
      browAngle: -0.2,
      eyeOpen: 0.0,
      eyeSmile: 0.0,
      mouthForm: -0.1,
      bodyAngleX: -1.0,
      bodyAngleY: -2.0,
      bodyAngleZ: -2.0,
      headAngleX: -12.0,
      headAngleY: -10.0,
      headAngleZ: -8.0,
      cheek: 0.0
    }
  };
  var frameworkRefCount = 0;
  function lerp(from, to, amount) {
    return from + (to - from) * amount;
  }
  function qBound(minV, value, maxV) {
    return Math.max(minV, Math.min(value, maxV));
  }
  function noise01(x, seed) {
    const s1 = Math.sin(x * 1.37 + seed * 0.71);
    const s2 = Math.sin(x * 2.11 + seed * 1.13);
    const s3 = Math.sin(x * 0.73 + seed * 2.03);
    return qBound(0.0, 0.5 + s1 * 0.28 + s2 * 0.16 + s3 * 0.06, 1.0);
  }
  function noiseSigned(x, seed) {
    return (noise01(x, seed) - 0.5) * 2.0;
  }
  function drift(phase, speed, seed, amplitude) {
    const slow = noiseSigned(phase * speed * 0.3, seed);
    const medium = noiseSigned(phase * speed * 0.8, seed + 50.0);
    const micro = noiseSigned(phase * speed * 2.5, seed + 100.0);
    return (slow * 0.5 + medium * 0.35 + micro * 0.15) * amplitude;
  }
  var MOTION_BURSTS = {
    NORMAL: { bodyX: 0, bodyY: 1, bodyZ: 0, headX: 0, headY: 1, headZ: 0, duration: 0.4, intensity: 0.5 },
    SMILE: { bodyX: 4, bodyY: 2, bodyZ: -2, headX: 5, headY: 3, headZ: -3, duration: 0.5, intensity: 0.8 },
    ANGRY: { bodyX: -5, bodyY: -3, bodyZ: 0, headX: -5, headY: -5, headZ: 0, duration: 0.5, intensity: 1.2 },
    SAD: { bodyX: -2, bodyY: -3, bodyZ: -2, headX: -3, headY: -4, headZ: -2, duration: 0.8, intensity: 0.7 },
    SURPRISED: { bodyX: -2, bodyY: 5, bodyZ: 2, headX: 0, headY: 8, headZ: 0, duration: 0.4, intensity: 1.5 },
    BLUSH: { bodyX: 8, bodyY: -2, bodyZ: -3, headX: 12, headY: -3, headZ: -5, duration: 0.6, intensity: 1.0 },
    WINK: { bodyX: 5, bodyY: 2, bodyZ: -2, headX: 6, headY: 3, headZ: -4, duration: 0.4, intensity: 0.9 },
    DISGUST: { bodyX: 5, bodyY: 4, bodyZ: 2, headX: 7, headY: -3, headZ: 3, duration: 0.5, intensity: 1.1 },
    SMUG: { bodyX: 2, bodyY: 2, bodyZ: -1, headX: 5, headY: 3, headZ: -1, duration: 0.7, intensity: 0.6 },
    THINKING: { bodyX: 0, bodyY: 0, bodyZ: 0, headX: 2, headY: -3, headZ: 1, duration: 0.8, intensity: 0.3 },
    PANIC: { bodyX: 0, bodyY: 0, bodyZ: 0, headX: 0, headY: 0, headZ: 0, duration: 0.2, intensity: 2.0 },
    SLEEPING: { bodyX: -1, bodyY: -2, bodyZ: -1, headX: -2, headY: -3, headZ: -2, duration: 4.5, intensity: 0.3 },
    SNEEZE: { bodyX: 0, bodyY: 0, bodyZ: 0, headX: 0, headY: 0, headZ: 0, duration: 2.2, intensity: 1.0 }
  };
  function getEmotionIdleMotion(tag, phase) {
    let bodyX = 0, bodyY = 0, bodyZ = 0;
    let headX = 0, headY = 0, headZ = 0;
    const t = tag.toUpperCase();
    if (t === "NORMAL") {
      bodyX = drift(phase, 0.6, 0.0, 1.8);
      bodyY = drift(phase, 0.4, 10.0, 0.8) + Math.sin(phase * 0.8) * 0.3;
      bodyZ = drift(phase, 0.35, 20.0, 0.6);
      headX = drift(phase, 0.5, 30.0, 3.0);
      headY = drift(phase, 0.4, 40.0, 2.0);
      headZ = drift(phase, 0.3, 50.0, 1.0);
    } else if (t === "SMILE") {
      bodyX = drift(phase, 1.2, 5.0, 3.0);
      bodyY = drift(phase, 1.0, 15.0, 1.5);
      bodyZ = drift(phase, 0.7, 25.0, 1.5);
      headX = drift(phase, 1.0, 35.0, 4.0);
      headY = drift(phase, 0.8, 45.0, 2.5);
      headZ = drift(phase, 0.9, 55.0, 2.0);
    } else if (t === "ANGRY") {
      const tension = noiseSigned(phase * 3.0, 7.0);
      bodyX = drift(phase, 1.5, 8.0, 2.5) + tension * 1.5;
      bodyY = drift(phase, 0.5, 18.0, 0.8);
      bodyZ = drift(phase, 2.0, 28.0, 1.2);
      headX = drift(phase, 1.8, 38.0, 3.0) + tension * 1.0;
      headY = drift(phase, 1.2, 48.0, 2.0);
      headZ = drift(phase, 0.8, 58.0, 1.0);
    } else if (t === "SAD") {
      bodyX = drift(phase, 0.4, 3.0, 2.5);
      bodyY = drift(phase, 0.3, 13.0, 1.5);
      bodyZ = drift(phase, 0.35, 23.0, 1.8);
      headX = drift(phase, 0.3, 33.0, 3.5);
      headY = drift(phase, 0.25, 43.0, 2.5);
      headZ = drift(phase, 0.3, 53.0, 1.5);
    } else if (t === "SURPRISED") {
      bodyX = drift(phase, 1.8, 6.0, 3.0);
      bodyY = drift(phase, 1.5, 16.0, 1.5) + Math.abs(Math.sin(phase * 1.5)) * 0.8;
      bodyZ = drift(phase, 1.0, 26.0, 1.5);
      headX = drift(phase, 2.0, 36.0, 5.0);
      headY = drift(phase, 1.8, 46.0, 4.5);
      headZ = drift(phase, 1.0, 56.0, 1.5);
    } else if (t === "BLUSH") {
      const fidget = noiseSigned(phase * 4.0, 99.0) * 0.75;
      bodyX = drift(phase, 0.8, 9.0, 3.0) + 1.0;
      bodyY = drift(phase, 0.6, 19.0, 1.0);
      bodyZ = drift(phase, 0.9, 29.0, 2.0);
      headX = drift(phase, 0.6, 39.0, 4.0) + 2.0 + fidget;
      headY = drift(phase, 0.5, 49.0, 3.0);
      headZ = drift(phase, 0.7, 59.0, 2.5);
    } else if (t === "WINK") {
      bodyX = drift(phase, 1.3, 4.0, 3.5);
      bodyY = drift(phase, 0.8, 14.0, 1.2);
      bodyZ = drift(phase, 0.9, 24.0, 2.0);
      headX = drift(phase, 1.0, 34.0, 4.5);
      headY = drift(phase, 0.7, 44.0, 2.5);
      headZ = drift(phase, 1.1, 54.0, 2.5);
    } else if (t === "DISGUST") {
      bodyX = drift(phase, 0.7, 2.0, 2.5) + 1.0;
      bodyY = drift(phase, 0.9, 12.0, 1.5);
      bodyZ = drift(phase, 0.5, 22.0, 1.0);
      headX = drift(phase, 0.8, 32.0, 3.5) + 1.5;
      headY = drift(phase, 0.6, 42.0, 2.5);
      headZ = drift(phase, 0.5, 52.0, 1.5);
    } else if (t === "SMUG") {
      bodyX = drift(phase, 0.5, 4.0, 2.0);
      bodyY = drift(phase, 0.4, 14.0, 1.0);
      bodyZ = drift(phase, 0.4, 24.0, 1.5);
      headX = drift(phase, 0.6, 34.0, 3.0) + 3.0;
      headY = drift(phase, 0.5, 44.0, 2.0);
      headZ = drift(phase, 0.5, 54.0, 1.0);
    } else if (t === "THINKING") {
      bodyX = drift(phase, 0.2, 5.0, 1.0);
      bodyY = drift(phase, 0.2, 15.0, 0.5);
      bodyZ = drift(phase, 0.2, 25.0, 0.5);
      headX = drift(phase, 0.2, 35.0, 1.0);
      headY = drift(phase, 0.2, 45.0, 1.0);
      headZ = drift(phase, 0.2, 55.0, 0.5);
    } else if (t === "PANIC") {
      const panic = noiseSigned(phase * 15.0, 999.0) * 2.0;
      bodyX = drift(phase, 2.0, 6.0, 2.0) + panic;
      bodyY = drift(phase, 2.0, 16.0, 2.0);
      bodyZ = drift(phase, 2.0, 26.0, 2.0);
      headX = drift(phase, 3.0, 36.0, 3.0) + panic;
      headY = panic * 1.5;
      headZ = panic * 0.5;
    } else if (t === "SLEEPING") {
      bodyX = drift(phase * 0.3, 0.15, 0.0, 0.5);
      bodyY = drift(phase * 0.3, 0.5, 10.0, 0.6) + Math.sin(phase * 0.8) * 0.6;
      bodyZ = drift(phase * 0.3, 0.1, 20.0, 0.3);
      headX = drift(phase * 0.3, 0.2, 30.0, 1.0);
      headY = drift(phase * 0.3, 0.15, 40.0, 0.5) + Math.sin(phase * 0.8) * 0.3;
      headZ = drift(phase * 0.3, 0.2, 50.0, 1.5);
    } else {
      bodyX = Math.sin(phase * 0.5) * 1.2;
      bodyY = Math.sin(phase * 0.3) * 0.6;
      bodyZ = Math.sin(phase * 0.4) * 0.8;
      headX = Math.sin(phase * 0.8) * 2.2;
      headY = Math.sin(phase * 0.6) * 1.5;
      headZ = Math.sin(phase * 0.7) * 0.9;
    }
    return { bodyX, bodyY, bodyZ, headX, headY, headZ };
  }
  function acquireFramework() {
    if (frameworkRefCount === 0) {
      const option = new Option();
      option.loggingLevel = 5 /* LogLevel_Off */;
      CubismFramework.startUp(option);
      CubismFramework.initialize();
    }
    frameworkRefCount += 1;
  }
  function releaseFramework() {
    if (frameworkRefCount <= 0) {
      return;
    }
    frameworkRefCount -= 1;
    if (frameworkRefCount === 0) {
      if (CubismFramework.isInitialized()) {
        CubismFramework.dispose();
      }
      if (CubismFramework.isStarted()) {
        CubismFramework.cleanUp();
      }
    }
  }
  function findExpressionName(model, tag) {
    if (!model || !model._expressions || model._expressions.size === 0) return null;
    const needle = tag.toLowerCase();
    let preferredKeys = [];
    if (needle === "sad") preferredKeys = ["sad"];
    else if (needle === "smile") preferredKeys = ["smile"];
    else if (needle === "angry") preferredKeys = ["angry"];
    else if (needle === "surprised") preferredKeys = ["surprised"];
    else if (needle === "blush") preferredKeys = ["blush"];
    else if (needle === "wink") preferredKeys = ["wink"];
    else if (needle === "disgust") preferredKeys = ["disgust"];
    else if (needle === "smug") preferredKeys = ["smug"];
    else if (needle === "thinking") preferredKeys = ["thinking"];
    else if (needle === "panic") preferredKeys = ["surprised"];
    else preferredKeys = [needle];

    let selectedName = null;
    for (const key of preferredKeys) {
      for (const name of model._expressions.keys()) {
        if (name.toLowerCase() === key) {
          selectedName = name;
          break;
        }
      }
      if (selectedName) break;
    }

    if (!selectedName) {
      for (const name of model._expressions.keys()) {
        if (name.toLowerCase().includes(needle)) {
          selectedName = name;
          break;
        }
      }
    }

    if (!selectedName) {
      selectedName = model._expressions.keys().next().value;
    }
    return selectedName;
  }
  var ReamaCubismPlayer = class {
    constructor(host) {
      this.subdelegate = null;
      this.rafHandle = null;
      this.parameterHandles = null;
      this.current = __spreadValues({}, EMOTION_TARGETS.NORMAL);
      this.userTime = 0;
      this.mouthOpen = 0;
      this.emotion = "NORMAL";
      this.lipSyncValue = 0;
      this.destroyed = false;
      this.frameworkAcquired = false;
      this.initRetryHandle = null;

      // Idle sleep & Easter Eggs
      this.wakeUpTimer = 0;
      this.wakeUpHeadX = 0;
      this.wakeUpGazeX = 0;
      this.sneezeEasterEggTimer = 0;
      this.burstProgress = 1.0;
      this.burstTimer = 0;
      this.activeBurstTag = "";
      this.sleepingNodTimer = 0;
      this.nextSleepingNodInterval = 30.0;
      this.spokenChar = "";
      this.currentEmotionTag = "NORMAL";
      this.idlePhase = 0;
      this.smoothedBodyX = 0;
      this.smoothedBodyY = 0;
      this.smoothedBodyZ = 0;
      this.smoothedHeadX = 0;
      this.smoothedHeadY = 0;
      this.smoothedHeadZ = 0;
      this.lastAppliedEmotionTag = "";
      this.eyeTrackingX = 0.0;
      this.eyeTrackingY = 0.0;
      this.smoothedGazeX = 0.0;
      this.smoothedGazeY = 0.0;

      // Blink FSM state
      this.blinkState = 0; // 0=Open, 1=Closing, 2=Opening
      this.blinkTimer = 0.0;
      this.blinkInterval = 3.0 + Math.random() * 3.5; // Random 2.5-6.0s
      this.blinkDuration = 0.1;

      this.tick = () => {
        if (this.destroyed || !this.subdelegate) {
          return;
        }
        LAppPal.updateTime();
        this.subdelegate.update();
        this.rafHandle = requestAnimationFrame(this.tick);
      };
      this.host = host;
      this.beginInitialize();
    }
    setEmotion(emotion) {
      const normalizedTag = emotion.trim().toUpperCase();
      if (normalizedTag === this.emotion) {
        return;
      }
      const isWakingUp = (this.emotion === "SLEEPING" && normalizedTag !== "SLEEPING");
      this.emotion = normalizedTag;

      if (isWakingUp) {
        this.wakeUpTimer = 2.5; // Startled wake-up duration (2.5 seconds)
        this.activeBurstTag = "SURPRISED";
        this.burstTimer = 0;
        this.burstProgress = 0.0;
        this.idlePhase = 0.0;
        this.currentEmotionTag = "SURPRISED";
      } else {
        this.wakeUpTimer = 0;
        if (normalizedTag !== this.currentEmotionTag) {
          this.activeBurstTag = normalizedTag;
          this.burstTimer = 0;
          this.burstProgress = 0.0;
          this.idlePhase = 0.0;
          this.currentEmotionTag = normalizedTag;
        }
      }
    }
    setSpokenChar(spokenChar) {
      this.spokenChar = spokenChar || "";
    }
    setLipSync(value) {
      this.lipSyncValue = Math.max(0, Math.min(1, value));
    }
    setEyeTracking(x, y) {
      this.eyeTrackingX = Math.max(-1.0, Math.min(1.0, x));
      this.eyeTrackingY = Math.max(-1.0, Math.min(1.0, y));
    }
    destroy() {
      this.destroyed = true;
      if (this.initRetryHandle !== null) {
        cancelAnimationFrame(this.initRetryHandle);
        this.initRetryHandle = null;
      }
      if (this.rafHandle !== null) {
        cancelAnimationFrame(this.rafHandle);
        this.rafHandle = null;
      }
      if (this.subdelegate) {
        this.subdelegate.getLive2DManager().setParameterDriver(null);
        this.subdelegate.release();
        this.subdelegate = null;
      }
      this.host.innerHTML = "";
      if (this.frameworkAcquired) {
        releaseFramework();
        this.frameworkAcquired = false;
      }
    }
    // Wait for the host element to have non-zero dimensions before initializing.
    // The browser may not have completed layout yet when the component mounts.
    beginInitialize() {
      const runtimeWindow2 = window;
      if (!runtimeWindow2.Live2DCubismCore) {
        console.error("[ReamaCubism] Live2DCubismCore is not loaded.");
        return;
      }
      const canvas = document.createElement("canvas");
      canvas.style.display = "block";
      canvas.style.width = "100%";
      canvas.style.height = "100%";
      canvas.style.pointerEvents = "none";
      this.host.innerHTML = "";
      this.host.appendChild(canvas);
      const tryInit = () => {
        if (this.destroyed) return;
        const w = canvas.clientWidth || this.host.clientWidth;
        const h = canvas.clientHeight || this.host.clientHeight;
        if (w > 0 && h > 0) {
          this.initSubdelegate(canvas);
        } else {
          this.initRetryHandle = requestAnimationFrame(tryInit);
        }
      };
      tryInit();
    }
    initSubdelegate(canvas) {
      this.initRetryHandle = null;
      acquireFramework();
      this.frameworkAcquired = true;
      console.log(`[ReamaCubism] Initializing. Canvas client size: ${canvas.clientWidth}x${canvas.clientHeight}`);
      const nextSubdelegate = new LAppSubdelegate();
      if (!nextSubdelegate.initialize(canvas)) {
        console.error("[ReamaCubism] Failed to initialize Cubism subdelegate.");
        this.host.innerHTML = "";
        releaseFramework();
        this.frameworkAcquired = false;
        return;
      }
      const gl = nextSubdelegate.getGl();
      if (gl) {
        const maxTex = gl.getParameter(gl.MAX_TEXTURE_SIZE);
        console.log(`[ReamaCubism] WebGL ready. Max texture size: ${maxTex}, Canvas buffer: ${gl.drawingBufferWidth}x${gl.drawingBufferHeight}`);
      }
      nextSubdelegate.getLive2DManager().setParameterDriver((model) => this.applyParameters(model));
      this.subdelegate = nextSubdelegate;
      this.tick();
    }
    resolveParameterHandles() {
      if (this.parameterHandles) {
        return this.parameterHandles;
      }
      const idManager = CubismFramework.getIdManager();
      this.parameterHandles = {
        browLY: idManager.getId("ParamBrowLY"),
        browRY: idManager.getId("ParamBrowRY"),
        browLForm: idManager.getId("ParamBrowLForm"),
        browRForm: idManager.getId("ParamBrowRForm"),
        browLAngle: idManager.getId("ParamBrowLAngle"),
        browRAngle: idManager.getId("ParamBrowRAngle"),
        eyeLOpen: idManager.getId("ParamEyeLOpen"),
        eyeROpen: idManager.getId("ParamEyeROpen"),
        eyeLSmile: idManager.getId("ParamEyeLSmile"),
        eyeRSmile: idManager.getId("ParamEyeRSmile"),
        mouthForm: idManager.getId("ParamMouthForm"),
        mouthOpenY: idManager.getId("ParamMouthOpenY"),
        bodyAngleX: idManager.getId("ParamBodyAngleX"),
        bodyAngleY: idManager.getId("ParamBodyAngleY"),
        bodyAngleZ: idManager.getId("ParamBodyAngleZ"),
        angleX: idManager.getId("ParamAngleX"),
        angleY: idManager.getId("ParamAngleY"),
        angleZ: idManager.getId("ParamAngleZ"),
        breath: idManager.getId("ParamBreath"),
        cheek: idManager.getId("ParamCheek"),
        eyeBallX: idManager.getId("ParamEyeBallX"),
        eyeBallY: idManager.getId("ParamEyeBallY")
      };
      return this.parameterHandles;
    }
    setParam(model, id, value) {
      const cubismModel = model.getModel();
      if (!cubismModel) {
        return;
      }
      cubismModel.setParameterValueById(id, value);
    }
    applyParameters(model) {
      var _a;
      if (this.destroyed) {
        return;
      }
      const cubismModel = model.getModel();
      if (!cubismModel) {
        return;
      }
      const ids = this.resolveParameterHandles();
      let target = (_a = EMOTION_TARGETS[this.emotion]) != null ? _a : EMOTION_TARGETS.NORMAL;
      const now = performance.now() / 1000.0;
      if (!this._lastUpdateTime) this._lastUpdateTime = now;
      const deltaSeconds = Math.min(now - this._lastUpdateTime, 0.1); // cap at 100ms
      this._lastUpdateTime = now;
      this.userTime += deltaSeconds;
      this.idlePhase += deltaSeconds;

      // Set initial/new expression once model is loaded
      if (model && model._expressions && model._expressions.size > 0) {
        if (this.lastAppliedEmotionTag !== this.currentEmotionTag) {
          const selectedName = findExpressionName(model, this.currentEmotionTag);
          if (selectedName) {
            model.setExpression(selectedName);
            this.lastAppliedEmotionTag = this.currentEmotionTag;
          }
        }
      }

      // 1. Sneeze Easter Egg (idle normal state, not speaking, not waking up, not sleeping)
      if (this.emotion === "NORMAL" && this.burstProgress >= 1.0 && this.wakeUpTimer <= 0.0 && this.lipSyncValue <= 0.01) {
        this.sneezeEasterEggTimer += deltaSeconds;
        if (this.sneezeEasterEggTimer >= 45.0) {
          this.sneezeEasterEggTimer = 0.0;
          if (Math.random() < 0.15) {
            this.activeBurstTag = "SNEEZE";
            this.burstTimer = 0.0;
            this.burstProgress = 0.0;
          }
        }
      } else {
        this.sneezeEasterEggTimer = 0.0;
      }

      // 2. Periodic sleep nodding
      if (this.emotion === "SLEEPING") {
        this.sleepingNodTimer += deltaSeconds;
        if (this.sleepingNodTimer >= this.nextSleepingNodInterval) {
          this.sleepingNodTimer = 0.0;
          this.nextSleepingNodInterval = 20.0 + Math.random() * 20.0; // 20 to 40 seconds
          this.activeBurstTag = "SLEEPING";
          this.burstProgress = 0.0;
          this.burstTimer = 0.0;
        }
      } else {
        this.sleepingNodTimer = 0.0;
      }

      // 3. Startled wake-up transition blend
      if (this.wakeUpTimer > 0.0) {
        this.wakeUpTimer -= deltaSeconds;
        if (this.wakeUpTimer <= 0.0) {
          this.wakeUpTimer = 0.0;
          this.wakeUpHeadX = 0.0;
          this.wakeUpGazeX = 0.0;
          this.currentEmotionTag = this.emotion; // Restore emotion tag when wakeUp timer expires
        } else {
          const targetBase = target;
          const targetSurprised = EMOTION_TARGETS.SURPRISED;
          let blend = 1.0;
          if (this.wakeUpTimer < 1.7) {
            blend = Math.max(0.0, Math.min(1.0, this.wakeUpTimer / 1.7));
          }

          target = {
            browY: lerp(targetBase.browY, targetSurprised.browY, blend),
            browForm: lerp(targetBase.browForm, targetSurprised.browForm, blend),
            browAngle: lerp(targetBase.browAngle, targetSurprised.browAngle, blend),
            eyeOpen: lerp(targetBase.eyeOpen, targetSurprised.eyeOpen, blend),
            eyeSmile: lerp(targetBase.eyeSmile, targetSurprised.eyeSmile, blend),
            mouthForm: lerp(targetBase.mouthForm, targetSurprised.mouthForm, blend),
            cheek: lerp(targetBase.cheek, targetSurprised.cheek, blend),
            bodyAngleX: lerp(targetBase.bodyAngleX, targetSurprised.bodyAngleX, blend),
            bodyAngleY: lerp(targetBase.bodyAngleY, targetSurprised.bodyAngleY, blend),
            bodyAngleZ: lerp(targetBase.bodyAngleZ, targetSurprised.bodyAngleZ, blend),
            headAngleX: lerp(targetBase.headAngleX, targetSurprised.headAngleX, blend),
            headAngleY: lerp(targetBase.headAngleY, targetSurprised.headAngleY, blend),
            headAngleZ: lerp(targetBase.headAngleZ, targetSurprised.headAngleZ, blend),
          };

          // Startled look around (look right, then look left)
          const elapsed = 2.5 - this.wakeUpTimer;
          if (elapsed >= 0.4 && elapsed < 1.4) {
            const wave = Math.sin((elapsed - 0.4) * (2.0 * Math.PI / 1.0));
            this.wakeUpHeadX = wave * 15.0; // Look right (+15), then look left (-15)
            this.wakeUpGazeX = wave * 0.7;  // Eyeball look right (+0.7), then left (-0.7)
          } else {
            this.wakeUpHeadX = 0.0;
            this.wakeUpGazeX = 0.0;
          }
        }
      } else {
        this.wakeUpHeadX = 0.0;
        this.wakeUpGazeX = 0.0;
      }

      // Smooth emotion blend
      const smooth = Math.min(deltaSeconds * 3.0, 1.0); // _emotionLerpSpeed = 3.0 in C++
      this.current = __spreadProps(__spreadValues({}, this.current), {
        browY: lerp(this.current.browY, target.browY, smooth),
        browForm: lerp(this.current.browForm, target.browForm, smooth),
        browAngle: lerp(this.current.browAngle, target.browAngle, smooth),
        eyeOpen: lerp(this.current.eyeOpen, target.eyeOpen, smooth),
        eyeSmile: lerp(this.current.eyeSmile, target.eyeSmile, smooth),
        mouthForm: lerp(this.current.mouthForm, target.mouthForm, smooth),
        bodyAngleX: lerp(this.current.bodyAngleX, target.bodyAngleX, smooth),
        bodyAngleY: lerp(this.current.bodyAngleY, target.bodyAngleY, smooth),
        bodyAngleZ: lerp(this.current.bodyAngleZ, target.bodyAngleZ, smooth),
        headAngleX: lerp(this.current.headAngleX, target.headAngleX, smooth),
        headAngleY: lerp(this.current.headAngleY, target.headAngleY, smooth),
        headAngleZ: lerp(this.current.headAngleZ, target.headAngleZ, smooth),
        cheek: lerp(this.current.cheek, target.cheek, smooth)
      });

      // 4. Handle Bursts (Nodding / Sneeze / Transitions)
      let burstBodyX = 0.0, burstBodyY = 0.0, burstBodyZ = 0.0;
      let burstHeadX = 0.0, burstHeadY = 0.0, burstHeadZ = 0.0;
      let sneezeMouthOpen = 0.0;
      let sneezeEyeOpen = 1.0;
      const isSneezeActive = (this.burstProgress < 1.0 && this.activeBurstTag === "SNEEZE");

      if (this.burstProgress < 1.0) {
        this.burstTimer += deltaSeconds;
        const activeBurst = MOTION_BURSTS[this.activeBurstTag] || MOTION_BURSTS.NORMAL;
        const duration = activeBurst.duration;
        this.burstProgress = Math.max(0.0, Math.min(1.0, this.burstTimer / duration));
        const t01 = this.burstProgress;

        if (this.activeBurstTag === "SLEEPING") {
          let nodCurve = 0.0;
          if (t01 < 0.33) {
            nodCurve = Math.sin((t01 / 0.33) * Math.PI) * -10.0;
          } else if (t01 < 0.66) {
            nodCurve = Math.sin(((t01 - 0.33) / 0.33) * Math.PI) * -10.0;
          } else {
            nodCurve = Math.sin(((t01 - 0.66) / 0.34) * Math.PI) * -18.0;
          }
          burstHeadY = nodCurve;
          burstBodyY = t01 * -1.5 + (nodCurve * 0.15);
        } else if (this.activeBurstTag === "SNEEZE") {
          if (t01 < 0.5) {
            let localT = t01 / 0.5;
            burstHeadY = localT * 12.0;
            burstHeadX = localT * -3.0;
            burstBodyY = localT * 1.5;
            sneezeEyeOpen = 1.0 - (localT * 0.9);
          } else if (t01 < 0.65) {
            let localT = (t01 - 0.5) / 0.15;
            burstHeadY = 12.0 - (localT * 32.0);
            burstHeadX = -3.0 + (localT * 6.0);
            burstBodyY = 1.5 - (localT * 5.0);
            sneezeEyeOpen = 0.1 * (1.0 - localT);
            sneezeMouthOpen = localT * 0.9;
          } else {
            let localT = (t01 - 0.65) / 0.35;
            burstHeadY = -20.0 + (localT * 20.0);
            burstHeadX = 3.0 - (localT * 3.0);
            burstBodyY = -3.5 + (localT * 3.5);
            sneezeEyeOpen = 0.0 + (localT * 1.0);
            sneezeMouthOpen = 0.9 * (1.0 - localT);
          }
        } else {
          // General spring-physics overshoot animation
          const spring = Math.sin(t01 * Math.PI * 2.5) * (1.0 - t01) * (1.0 - t01);
          const intensity = activeBurst.intensity * spring;
          burstBodyX = activeBurst.bodyX * intensity;
          burstBodyY = activeBurst.bodyY * intensity;
          burstBodyZ = activeBurst.bodyZ * intensity;
          burstHeadX = activeBurst.headX * intensity;
          burstHeadY = activeBurst.headY * intensity;
          burstHeadZ = activeBurst.headZ * intensity;
        }
      }

      // Blink & Eyes
      // Blink FSM (matching QT C++ UpdateBlink)
      this.blinkTimer += deltaSeconds;
      let blinkValue = 1.0; // fully open
      if (this.blinkState === 0) { // Open
        if (this.blinkTimer >= this.blinkInterval) {
          this.blinkState = 1; // start closing
          this.blinkTimer = 0.0;
        }
      } else if (this.blinkState === 1) { // Closing
        const t = Math.min(this.blinkTimer / this.blinkDuration, 1.0);
        blinkValue = 1.0 - t;
        if (this.blinkTimer >= this.blinkDuration) {
          this.blinkState = 2; // start opening
          this.blinkTimer = 0.0;
        }
      } else if (this.blinkState === 2) { // Opening
        const t = Math.min(this.blinkTimer / this.blinkDuration, 1.0);
        blinkValue = t;
        if (this.blinkTimer >= this.blinkDuration) {
          this.blinkState = 0; // back to open
          this.blinkTimer = 0.0;
          this.blinkInterval = 2.5 + Math.random() * 3.5;
        }
      }
      const blink = blinkValue;
      let nodEyeOpen = 1.0;
      if (this.emotion === "SLEEPING" && this.burstProgress < 1.0) {
        const t01 = this.burstProgress;
        let localT = 0.0;
        if (t01 < 0.33) {
          localT = t01 / 0.33;
        } else if (t01 < 0.66) {
          localT = (t01 - 0.33) / 0.33;
        } else {
          localT = (t01 - 0.66) / 0.34;
        }
        nodEyeOpen = 0.18 * Math.sin(localT * Math.PI);
      }
      let baseEyeOpen = this.current.eyeOpen;
      if (this.emotion === "SLEEPING" && this.burstProgress < 1.0) {
        baseEyeOpen = nodEyeOpen;
      } else if (isSneezeActive) {
        baseEyeOpen = sneezeEyeOpen;
      }
      const eyeOpen = Math.max(0.0, baseEyeOpen * blink);

      // Set parameter values
      this.setParam(model, ids.browLY, this.current.browY);
      this.setParam(model, ids.browRY, this.current.browY);
      this.setParam(model, ids.browLForm, this.current.browForm);
      this.setParam(model, ids.browRForm, this.current.browForm);
      this.setParam(model, ids.browLAngle, this.current.browAngle);
      this.setParam(model, ids.browRAngle, this.current.browAngle);
      this.setParam(model, ids.eyeLSmile, this.current.eyeSmile);
      this.setParam(model, ids.eyeRSmile, this.current.eyeSmile);
      this.setParam(model, ids.mouthForm, this.current.mouthForm);
      this.setParam(model, ids.cheek, this.current.cheek);
      this.setParam(model, ids.eyeLOpen, eyeOpen);
      this.setParam(model, ids.eyeROpen, target.isWink ? 0 : eyeOpen);

      // 5. Idle movement (breathing & drift) with smoothing
      const idleMotion = getEmotionIdleMotion(this.currentEmotionTag, this.idlePhase);
      const motionSmoothT = Math.max(0.0, Math.min(deltaSeconds * 6.0, 1.0));
      this.smoothedBodyX = lerp(this.smoothedBodyX, burstBodyX + idleMotion.bodyX, motionSmoothT);
      this.smoothedBodyY = lerp(this.smoothedBodyY, burstBodyY + idleMotion.bodyY, motionSmoothT);
      this.smoothedBodyZ = lerp(this.smoothedBodyZ, burstBodyZ + idleMotion.bodyZ, motionSmoothT);
      this.smoothedHeadX = lerp(this.smoothedHeadX, burstHeadX + idleMotion.headX + this.wakeUpHeadX, motionSmoothT);
      this.smoothedHeadY = lerp(this.smoothedHeadY, burstHeadY + idleMotion.headY, motionSmoothT);
      this.smoothedHeadZ = lerp(this.smoothedHeadZ, burstHeadZ + idleMotion.headZ, motionSmoothT);
      // Gaze tracking (eyeball look) — compute smoothed gaze before body/head params (QT parity)
      const disableGaze = (this.wakeUpTimer > 0.0) || (this.emotion === "SLEEPING") || (this.currentEmotionTag === "SLEEPING");
      const targetGazeX = this.wakeUpTimer > 0.0 ? this.wakeUpGazeX : (disableGaze ? 0.0 : (this.eyeTrackingX || 0.0));
      const targetGazeY = disableGaze ? 0.0 : (this.eyeTrackingY || 0.0);

      const gazeSmoothT = Math.max(0.0, Math.min(1.0, deltaSeconds * 5.0));
      this.smoothedGazeX = lerp(this.smoothedGazeX, targetGazeX, gazeSmoothT);
      this.smoothedGazeY = lerp(this.smoothedGazeY, targetGazeY, gazeSmoothT);

      this.setParam(model, ids.bodyAngleX, this.current.bodyAngleX + this.smoothedBodyX + (this.smoothedGazeX * 5.0));
      this.setParam(model, ids.bodyAngleY, this.current.bodyAngleY + this.smoothedBodyY + (this.smoothedGazeY * 5.0));
      this.setParam(model, ids.bodyAngleZ, this.current.bodyAngleZ + this.smoothedBodyZ);
      this.setParam(model, ids.angleX, this.current.headAngleX + this.smoothedHeadX + (this.smoothedGazeX * 25.0));
      this.setParam(model, ids.angleY, this.current.headAngleY + this.smoothedHeadY + (this.smoothedGazeY * 25.0));
      this.setParam(model, ids.angleZ, this.current.headAngleZ + this.smoothedHeadZ);

      this.setParam(model, ids.eyeBallX, this.smoothedGazeX);
      this.setParam(model, ids.eyeBallY, this.smoothedGazeY);

      // Breath cycles
      const breathCycleSpeed = this.emotion === "SLEEPING" ? 0.8 : 1.2;
      this.setParam(model, ids.breath, (Math.sin(this.userTime * breathCycleSpeed) + 1) * 0.5);

      // 6. Lip sync & Vowel mouth opening
      if (isSneezeActive) {
        this.mouthOpen = lerp(this.mouthOpen, sneezeMouthOpen, 16.0 * deltaSeconds);
      } else if (this.lipSyncValue > 0.01) {
        let targetMouth = 0.5;
        if (this.spokenChar) {
          targetMouth = getVowelMouthOpening(this.spokenChar);
        } else {
          const syllable = Math.abs(Math.sin(this.userTime * 12));
          const subBeat = Math.abs(Math.sin(this.userTime * 7.3));
          const flutter = noise01(this.userTime * 8.0, 5.0);
          targetMouth = syllable * 0.5 + subBeat * 0.3 + flutter * 0.2;
        }
        const finalMouth = Math.max(0.0, Math.min(1.0, targetMouth * Math.max(0.0, Math.min(1.0, this.lipSyncValue))));
        this.mouthOpen = lerp(this.mouthOpen, finalMouth, 14.0 * deltaSeconds);
      } else {
        this.mouthOpen = lerp(this.mouthOpen, 0.0, 10.0 * deltaSeconds);
      }
      this.setParam(model, ids.mouthOpenY, this.mouthOpen);

      // Helper function definitions
      function getVowelMouthOpening(chr) {
        if (!chr) return 0.0;
        const c = chr.charAt(0);

        if (c === 'a' || c === 'A') return 0.9;
        if (c === 'o' || c === 'O') return 0.7;
        if (c === 'e' || c === 'E') return 0.6;
        if (c === 'u' || c === 'U') return 0.4;
        if (c === 'i' || c === 'I' || c === 'í' || c === 'Í') return 0.2;

        const aVowels = "あかさたなはまやらわがざだばぱぁゃァカサタナハマヤラワガザダバパァャ";
        if (aVowels.includes(c)) return 0.9;

        const oVowels = "おこそとのほもよろをごぞどぼぽぉょォコソトノホモヨロヲゴゾドボポォョ";
        if (oVowels.includes(c)) return 0.7;

        const eVowels = "えけせてねへめれげぜでべぺぇェケセテネヘメレゲゼデベペェ";
        if (eVowels.includes(c)) return 0.6;

        const uVowels = "うくすつぬふむゆるぐずづぶぷぅゅゥクスツヌフムユルグズヅブプゥュ";
        if (uVowels.includes(c)) return 0.4;

        const iVowels = "いきしちにひみりぎじぢびぴぃィキシチニヒミリギジヂビピィ";
        if (iVowels.includes(c)) return 0.2;

        const closedChars = "んン。、？！.,?! 　\n\r\t-_";
        if (closedChars.includes(c)) return 0.0;

        return 0.5;
      }
    }
  };
  function preloadDefaultAssets() {
    const modelDir = ResourcesPath + ModelDir[0] + "/";
    const modelJsonUrl = modelDir + ModelJsonFileName;
    const shaderPaths = [
      VertShaderSrcPath,
      VertShaderSrcMaskedPath,
      VertShaderSrcSetupMaskPath,
      FragShaderSrcSetupMaskPath,
      FragShaderSrcPremultipliedAlphaPath,
      FragShaderSrcMaskPremultipliedAlphaPath,
      FragShaderSrcMaskInvertedPremultipliedAlphaPath,
      VertShaderSrcCopyPath,
      FragShaderSrcCopyPath,
      FragShaderSrcColorBlendPath,
      FragShaderSrcAlphaBlendPath,
      VertShaderSrcBlendPath,
      FragShaderSrcBlendPath
    ];
    const tasks = [
      cachedFetchArrayBuffer(modelJsonUrl).catch(() => {}),
      cachedFetchArrayBuffer(modelDir + "Live2D\u7D05\u8389\u6816forSDK5.0.moc3").catch(() => {}),
      cachedFetchArrayBuffer(modelDir + "Live2D\u7D05\u8389\u6816forSDK5.0.physics3.json").catch(() => {}),
      cachedLoadImage(modelDir + "Live2D\u7D05\u8389\u6816forSDK5.0.8192/texture_00.png").catch(() => {})
    ];
    for (const sp of shaderPaths) {
      tasks.push(cachedFetchText(ShaderPath + sp).catch(() => {}));
    }
    return Promise.all(tasks).then(() => {});
  }
  var runtime = {
    create(host) {
      return new ReamaCubismPlayer(host);
    },
    preload: preloadDefaultAssets
  };
  var runtimeWindow = window;
  runtimeWindow.ReamaCubismRuntime = runtime;
  void preloadDefaultAssets();
  window.__internals = { LAppModel, LAppLive2DManager, LAppPal, CubismFramework, ReamaCubismPlayer, CubismMatrix44, CubismWebGLOffscreenManager, acquireFramework, releaseFramework, CubismUserModel, LAppView, CubismViewMatrix, EMOTION_TARGETS, MOTION_BURSTS, noise01, drift, getEmotionIdleMotion, s_arrayBufferCache, s_textCache, s_imageCache };
})();
