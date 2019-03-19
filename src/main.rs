struct Module {
    binary: Vec<u8>,
}

const TYPE_SECTION_ID: u32 = 1;

impl Module {
    fn new(binary: Vec<u8>) -> Self {
        Self { binary }
    }

    fn check_header(&self, cur: &mut usize) -> bool {
        if self.binary[*cur] != 0 ||
            self.binary[*cur + 1] != 97 ||  // A
            self.binary[*cur + 2] != 115 || // S
            self.binary[*cur + 3] != 109
        {
            // M
            println!("error when checking header: expected 0xasm");
            return false;
        }

        if self.binary[*cur + 4] != 1
            || self.binary[*cur + 5] != 0
            || self.binary[*cur + 6] != 0
            || self.binary[*cur + 6] != 0
        {
            println!("error when checking header: expected version 0x1");
            false
        } else {
            *cur += 8;
            true
        }
    }

    fn read_var_u32(&self, cur: &mut usize) -> u32 {
        // Optimization for single byte i32.
        let byte = self.binary[*cur] as u32;
        *cur += 1;
        if (byte & 0x80) == 0 {
            return byte;
        }

        let mut result = byte & 0x7F;
        let mut shift = 7;
        loop {
            let byte = self.binary[*cur] as u32;
            result |= ((byte & 0x7F) as u32) << shift;
            if shift >= 25 && (byte >> (32 - shift)) != 0 {
                // The continuation bit or unused bits are set.
                panic!("invalid var_u32");
            }
            *cur += 1;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
        }
        return result;
    }

    fn read_section_header(&self, cur: &mut usize, expected_section: u32) -> bool {
        let id = self.binary[*cur];
        if id != expected_section as u8 {
            println!("unexpected section {}", id);
            return false;
        }

        *cur += 1;

        let size = self.read_var_u32(cur);
        return true;
    }

    fn decode_func(&self, cur: &mut usize) {
        let num_args = self.read_var_u32(cur);
        for i in 0..num_args {
            let valtype_code = self.binary[*cur];
            *cur += 1;
        }

        let num_rets = self.read_var_u32(cur);
        for i in 0..num_rets {
            let valtype_code = self.binary[*cur];
            *cur += 1;
        }
        println!(
            "function with {} args and {} return values",
            num_args, num_rets
        );
    }

    fn call(&self, func_index: usize) -> Result<(), &'static str> {
        let mut cur = 0;
        if !self.check_header(&mut cur) {
            return Err("checking header failed");
        }

        if !self.read_section_header(&mut cur, TYPE_SECTION_ID) {
            return Err("type section not found");
        }

        let num_types = self.read_var_u32(&mut cur);
        println!("found {} types", num_types);

        let mut i = 0;
        while i < num_types {
            let type_code = self.read_var_u32(&mut cur);
            if type_code != 0x60 {
                println!("unsupported type code");
            }
            self.decode_func(&mut cur);
            i += 1;
        }

        // Function declaration section.
        self.read_section_header(&mut cur, 3);
        let num_defs = self.read_var_u32(&mut cur);
        for i in 0..num_defs {
            let func_type_index = self.read_var_u32(&mut cur);
            if func_type_index > num_types {
                return Err("unexpected function signature index");
            }
        }

        // Export section.
        self.read_section_header(&mut cur, 7);
        let num_exports = self.read_var_u32(&mut cur);
        for i in 0..num_exports {
            // skip name
            let num_bytes = self.read_var_u32(&mut cur);
            cur += num_bytes as usize;

            // export kind
            let export_kind = self.read_var_u32(&mut cur);
            if export_kind != 0 {
                println!("not a function");
            }

            // TODO func_index is a var u32, check it's correct,
            let func_index = self.read_var_u32(&mut cur);
        }

        if !self.read_section_header(&mut cur, 10) {
            println!("was expecting code section");
            return Err("expected code section");
        }

        // TODO implement code section

        Ok(())
    }
}

fn main() {
    let binary = vec![
        0, 97, 115, 109, 1, 0, 0, 0, 1, 133, 128, 128, 128, 0, 1, 96, 0, 1, 127, 3, 130, 128, 128,
        128, 0, 1, 0, 7, 135, 128, 128, 128, 0, 1, 3, 97, 100, 100, 0, 0, 10, 141, 128, 128, 128,
        0, 1, 135, 128, 128, 128, 0, 0, 65, 1, 65, 2, 106, 11,
    ];

    let m = Module::new(binary);
    m.call(0);
}
