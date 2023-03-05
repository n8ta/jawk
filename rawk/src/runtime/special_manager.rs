use crate::awk_str::RcAwkStr;
use crate::parser::SclSpecial;
use crate::runtime::VmRuntime;
use crate::specials::NUM_SCL_SPECIALS;
use crate::util::unwrap;
use crate::vm::{RuntimeScalar, VirtualMachine};

// Manages getting and setting awk specials
pub struct SpecialManager {
    storage: Vec<RuntimeScalar>,
}

impl SpecialManager {
    pub fn new(argc: usize) -> Self {
        // TODO: I could speed this up by hard coding the init values but it's
        // only run once so not vital
        let storage = SclSpecial::variants().iter().map(|(name, special)| {
            match special {
                SclSpecial::FS => RuntimeScalar::Str(RcAwkStr::new_str(" ")),
                SclSpecial::RS => RuntimeScalar::Str(RcAwkStr::new_str("\n")),
                SclSpecial::FILENAME => RuntimeScalar::Str(RcAwkStr::new_str("-")),
                SclSpecial::FNR => RuntimeScalar::Num(0.0),
                SclSpecial::NF => RuntimeScalar::Num(0.0),
                SclSpecial::NR => RuntimeScalar::Num(0.0),
                SclSpecial::CONVFMT => RuntimeScalar::Str(RcAwkStr::new_str("%.6g")),
                SclSpecial::OFMT => RuntimeScalar::Str(RcAwkStr::new_str("%.6g")),
                // TODO: OFS ORS
                SclSpecial::OFS => RuntimeScalar::Str(RcAwkStr::new_str("PRINT OFS SEP NOT YET IMPLEMENTED")),
                SclSpecial::ORS => RuntimeScalar::Str(RcAwkStr::new_str("PRINT ORS SEP NOT YET IMPLEMENTED")),
                SclSpecial::RLENGTH => RuntimeScalar::Num(0.0),
                SclSpecial::RSTART => RuntimeScalar::Num(0.0),
                SclSpecial::SUBSEP => RuntimeScalar::Str(RcAwkStr::new_str("-")),
                SclSpecial::ARGC => RuntimeScalar::Num(argc as f64),
            }
        }).collect();
        Self { storage }
    }


    // Converts a runtime scalar to its string representation (if it isn't already)
    // using internal number to string conversion
    fn scalar_to_string_internal(rt: &mut VmRuntime, scalar: RuntimeScalar) -> Vec<u8> {
        match scalar {
            RuntimeScalar::Str(str) => str.downgrade_or_clone_to_vec(),
            RuntimeScalar::StrNum(str) => str.downgrade_or_clone_to_vec(),
            RuntimeScalar::Num(num) => {
                let str = rt.converter.num_to_str_internal(num);
                str.to_vec()
            }
        }
    }

    pub fn assign(&mut self, special: SclSpecial, value: RuntimeScalar, rt: &mut VmRuntime) -> RuntimeScalar {
        let existing = unwrap(self.storage.get_mut(special as usize));
        let prior_value = std::mem::replace(existing, value.clone());
        match special {
            SclSpecial::FS => {
                let fs = SpecialManager::scalar_to_string_internal(rt, value);
                rt.columns.set_fs(fs);
            }
            SclSpecial::RS => {
                let rs = SpecialManager::scalar_to_string_internal(rt, value);
                rt.columns.set_rs(rs);
            }

            // Nothing to be done for these
            SclSpecial::SUBSEP | SclSpecial::ARGC
            | SclSpecial::FNR | SclSpecial::NF
            | SclSpecial::NR | SclSpecial::RLENGTH
            | SclSpecial::RSTART => {}

            SclSpecial::FILENAME => todo!("scl special manager"),
            SclSpecial::CONVFMT => todo!("scl special manager"),
            SclSpecial::OFMT => todo!("scl special manager"),
            SclSpecial::OFS => todo!("scl special manager"),
            SclSpecial::ORS => todo!("scl special manager"),
        }
        prior_value
    }

    pub fn get(&self, special: SclSpecial) -> RuntimeScalar {
        unwrap(self.storage.get(special as usize)).clone()
    }
}