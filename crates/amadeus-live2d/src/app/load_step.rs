//! `LoadStep` — the `LAppModel` load state machine values (pipeline spec §6.3; the numeric values
//! are kept for debugging parity with V1's `_state`).

/// `LAppModel._state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum LoadStep {
    #[default]
    LoadAssets = 0,
    LoadModel = 1,
    WaitLoadModel = 2,
    LoadExpression = 3,
    WaitLoadExpression = 4,
    LoadPhysics = 5,
    WaitLoadPhysics = 6,
    LoadPose = 7,
    WaitLoadPose = 8,
    SetupEyeBlink = 9,
    SetupBreath = 10,
    LoadUserData = 11,
    WaitLoadUserData = 12,
    SetupEyeBlinkIds = 13,
    SetupLipSyncIds = 14,
    SetupLook = 15,
    SetupLayout = 16,
    LoadMotion = 17,
    WaitLoadMotion = 18,
    CompleteInitialize = 19,
    CompleteSetupModel = 20,
    LoadTexture = 21,
    WaitLoadTexture = 22,
    /// The only state in which `update()` / `draw()` act.
    CompleteSetup = 23,
}

impl LoadStep {
    /// The V1 numeric value.
    pub fn value(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::LoadStep;

    #[test]
    fn numeric_values_match_v1() {
        assert_eq!(LoadStep::LoadAssets.value(), 0);
        assert_eq!(LoadStep::WaitLoadPhysics.value(), 6);
        assert_eq!(LoadStep::SetupLayout.value(), 16);
        assert_eq!(LoadStep::LoadTexture.value(), 21);
        assert_eq!(LoadStep::WaitLoadTexture.value(), 22);
        assert_eq!(LoadStep::CompleteSetup.value(), 23);
        assert!(LoadStep::WaitLoadModel < LoadStep::CompleteSetup);
    }
}
