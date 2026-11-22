| idx | Id | cdi3 Name | min | max | default | written by (per frame, in order) |
|---|---|---|---|---|---|---|
| 0 | `ParamAngleX` | 角度 X | -30 | 30 | 0 | Look(+0), Breath(+) → Physics input; **overwritten by driver** (head X) |
| 1 | `ParamAngleY` | 角度 Y | -30 | 30 | 0 | Look(+0), Breath(+); **overwritten by driver** (head Y) |
| 2 | `ParamAngleZ` | 角度 Z | -30 | 30 | 0 | Look(+0), Breath(+) → Physics input; **overwritten by driver** (head Z) |
| 3 | `ParamEyeLOpen` | 左目　開閉 | 0 | 1 | 1 | driver (eyeOpen × blink) |
| 4 | `ParamEyeLSmile` | 左目　笑顔 | 0 | 1 | 0 | driver |
| 5 | `ParamEyeROpen` | 右目　開閉 | 0 | 1 | 1 | driver (0 when target.isWink) |
| 6 | `ParamEyeRSmile` | 右目　笑顔 | 0 | 1 | 0 | driver |
| 7 | `ParamEyeBallX` | 目玉 X | -1 | 1 | 0 | Look(+0); **driver** (smoothed gaze X) |
| 8 | `ParamEyeBallY` | 目玉 Y | -1 | 1 | 0 | Look(+0); **driver** (smoothed gaze Y) |
| 9 | `ParamBrowLForm` | 左眉　変形 | -1 | 1 | 0 | driver |
| 10 | `ParamBrowLY` | 左眉　上下 | -1 | 1 | 0 | driver |
| 11 | `ParamBrowRAngle` | 右眉　角度 | -1 | 1 | 0 | driver |
| 12 | `ParamBrowLX` | 左眉　左右 | -1 | 1 | 0 | nobody (stays default via loadParameters) |
| 13 | `ParamBrowRX` | 右眉　左右 | -1 | 1 | 0 | nobody (stays default) |
| 14 | `ParamBrowLAngle` | 左眉　角度 | -1 | 1 | 0 | driver |
| 15 | `ParamBrowRY` | 右眉　上下 | -1 | 1 | 0 | driver |
| 16 | `ParamBrowRForm` | 右眉　変形 | -1 | 1 | 0 | driver |
| 17 | `ParamMouthForm` | 口　変形 | -1 | 1 | 0 | driver |
| 18 | `ParamMouthOpenY` | 口　開閉 | 0 | 1 | 0 | driver (lip sync) |
| 19 | `ParamCheek` | 照れ | 0 | 1 | 0 | driver |
| 20 | `ParamBodyAngleX` | 体の回転 X | -10 | 10 | 0 | Look(+0), Breath(+) → Physics input; **overwritten by driver** |
| 21 | `ParamBodyAngleY` | 体の回転 Y | -10 | 10 | 0 | driver |
| 22 | `ParamBodyAngleZ` | 体の回転 Z | -10 | 10 | 0 | Physics input (default value only); **overwritten by driver** |
| 23 | `ParamBreath` | 呼吸 | 0 | 1 | 0 | Breath(+); **overwritten by driver** |
| 24 | `ParamHairFront` | 髪揺れ　前 | -1 | 1 | 0 | Physics output (PhysicsSetting1) |
| 25 | `ParamHairSide` | 髪揺れ　横 | -1 | 1 | 0 | Physics output (PhysicsSetting2) |
| 26 | `ParamHairBack` | 髪揺れ　後 | -1 | 1 | 0 | Physics output (PhysicsSetting3) |
