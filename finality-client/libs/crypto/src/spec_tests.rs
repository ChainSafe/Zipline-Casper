use alloc::string::ToString;
use ethereum_consensus::crypto::{verify_signature, PublicKey, Signature};
use ethereum_consensus::primitives::Bytes32;
use serde::Deserialize;
use serde_with::{serde_as, DefaultOnError};
use test_utils::{load_yaml, TestCase};

#[serde_as]
#[derive(Debug, Deserialize)]
struct VerifyInput {
    #[serde_as(deserialize_as = "DefaultOnError")]
    pubkey: Option<PublicKey>,
    message: Bytes32,
    #[serde_as(deserialize_as = "DefaultOnError")]
    signature: Option<Signature>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyTestCase {
    input: VerifyInput,
    output: bool,
}

impl VerifyTestCase {
    pub fn from(test_case_path: &str) -> Self {
        let path = test_case_path.to_string() + "/data.yaml";
        load_yaml(&path)
    }

    fn run(&self) -> bool {
        verify_signature(
            self.input.pubkey.as_ref().unwrap(),
            self.input.message.as_ref(),
            self.input.signature.as_ref().unwrap(),
        )
        .is_ok()
    }
}

impl TestCase for VerifyTestCase {
    fn should_succeed(&self) -> bool {
        self.output
    }

    fn verify_success(&self) -> bool {
        self.run()
    }

    fn verify_failure(&self) -> bool {
        if self.input.signature.is_none() {
            return true;
        }
        if self.input.pubkey.is_none() {
            return true;
        }
        !self.run()
    }
}

macro_rules! test_path {
    ($t:literal) => {
        concat!(
            "../../../consensus-spec-tests/tests/general/phase0/bls/verify/small/",
            $t
        )
    };
}

#[test]
fn test_verify_infinity_pubkey_and_infinity_signature() {
    let test_case =
        VerifyTestCase::from(test_path!("verify_infinity_pubkey_and_infinity_signature"));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_195246_ee_3_bd_3_b_6_ec() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_195246ee3bd3b6ec"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_2_ea_479_adf_8_c_40300() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_2ea479adf8c40300"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_2_f_09_d_443_ab_8_a_3_ac_2() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_2f09d443ab8a3ac2"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_3208262581_c_8_fc_09() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_3208262581c8fc09"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_6_b_3_b_17_f_6962_a_490_c() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_6b3b17f6962a490c"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_6_eeb_7_c_52_dfd_9_baf_0() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_6eeb7c52dfd9baf0"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_8761_a_0_b_7_e_920_c_323() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_8761a0b7e920c323"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_d_34885_d_766_d_5_f_705() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_d34885d766d5f705"
    ));

    test_case.execute();
}

#[test]
fn test_verify_tampered_signature_case_e_8_a_50_c_445_c_855360() {
    let test_case = VerifyTestCase::from(test_path!(
        "verify_tampered_signature_case_e8a50c445c855360"
    ));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_195246_ee_3_bd_3_b_6_ec() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_195246ee3bd3b6ec"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_2_ea_479_adf_8_c_40300() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_2ea479adf8c40300"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_2_f_09_d_443_ab_8_a_3_ac_2() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_2f09d443ab8a3ac2"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_3208262581_c_8_fc_09() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_3208262581c8fc09"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_6_b_3_b_17_f_6962_a_490_c() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_6b3b17f6962a490c"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_6_eeb_7_c_52_dfd_9_baf_0() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_6eeb7c52dfd9baf0"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_8761_a_0_b_7_e_920_c_323() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_8761a0b7e920c323"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_d_34885_d_766_d_5_f_705() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_d34885d766d5f705"));

    test_case.execute();
}

#[test]
fn test_verify_valid_case_e_8_a_50_c_445_c_855360() {
    let test_case = VerifyTestCase::from(test_path!("verify_valid_case_e8a50c445c855360"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_195246_ee_3_bd_3_b_6_ec() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_195246ee3bd3b6ec"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_2_ea_479_adf_8_c_40300() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_2ea479adf8c40300"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_2_f_09_d_443_ab_8_a_3_ac_2() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_2f09d443ab8a3ac2"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_3208262581_c_8_fc_09() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_3208262581c8fc09"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_6_b_3_b_17_f_6962_a_490_c() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_6b3b17f6962a490c"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_6_eeb_7_c_52_dfd_9_baf_0() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_6eeb7c52dfd9baf0"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_8761_a_0_b_7_e_920_c_323() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_8761a0b7e920c323"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_d_34885_d_766_d_5_f_705() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_d34885d766d5f705"));

    test_case.execute();
}

#[test]
fn test_verify_wrong_pubkey_case_e_8_a_50_c_445_c_855360() {
    let test_case = VerifyTestCase::from(test_path!("verify_wrong_pubkey_case_e8a50c445c855360"));

    test_case.execute();
}
