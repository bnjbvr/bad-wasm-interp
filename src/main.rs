struct Module {
    binary: Vec<u8>,
}

enum EvalResult {
    I32(i32),
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
        // M
        {
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

    fn decode_type_section(&self, mut cur: &mut usize) -> Result<u32, &'static str> {
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

        Ok(num_types)
    }

    fn decode_func_decl(&self, num_func_types: u32, mut cur: &mut usize) -> usize {
        self.read_section_header(&mut cur, 3);
        let num_decls = self.read_var_u32(&mut cur);
        for i in 0..num_decls {
            let func_type_index = self.read_var_u32(&mut cur);
            if func_type_index > num_func_types {
                println!("unexpected function signature index");
                return usize::max_value();
            }
        }
        num_decls as usize
    }

    fn decode_exports_section(&self, num_decls: u32, mut cur: &mut usize) {
        self.read_section_header(&mut cur, 7);
        let num_exports = self.read_var_u32(&mut cur);
        for i in 0..num_exports {
            // skip name
            let num_bytes = self.read_var_u32(&mut cur);
            *cur += num_bytes as usize;

            // export kind
            let export_kind = self.read_var_u32(&mut cur);
            if export_kind != 0 {
                println!("not a function");
            }

            let func_index = self.read_var_u32(&mut cur);
            if func_index > num_decls {
                println!("one exported function doesn't exist!");
            }
        }
    }

    fn decode_func_body(&self, must_interpret: bool, mut cur: &mut usize) -> Option<EvalResult> {
        let body_size = self.read_var_u32(cur);
        if !must_interpret {
            // skip the bytes.
            *cur += body_size as usize;
            return None;
        }

        let num_local_groups = self.read_var_u32(cur);
        for i in 0..num_local_groups {
            let group_count = self.read_var_u32(cur);
            // skip code (one u8).
            *cur += 1;
        }

        let mut virtual_stack = Vec::new();

        loop {
            if *cur + 1 > self.binary.len() {
                println!("error: exhausted bytes before reaching an End");
            }

            let opcode = self.binary[*cur];
            *cur += 1;

            match opcode {
                0x01 => {
                    // Nope nope nope.
                }
                0x0b => {
                    // End opcode: terminate current block/loop.
                    let result = virtual_stack.pop().expect("missing result");
                    // TODO check that there aren't unused operands on the virtual stack.
                    return Some(EvalResult::I32(result));
                }
                0x41 => {
                    // i32.const
                    let num = self.read_var_u32(cur) as i32;
                    virtual_stack.push(num);
                }
                0x6a => {
                    // i32.add
                    let a = virtual_stack.pop().expect("missing operand");
                    let b = virtual_stack.pop().unwrap();
                    let num = a + b;
                    virtual_stack.push(num);
                }
                _ => unreachable!(format!("unknown opcode: {}", opcode)),
            }
        }
    }

    fn decode_code_section(
        &self,
        callee_index: u32,
        mut cur: &mut usize,
    ) -> Result<(), &'static str> {
        if !self.read_section_header(&mut cur, 10) {
            println!("was expecting code section");
            return Err("expected code section");
        }

        let num_func_defs = self.read_var_u32(&mut cur);
        // TODO does the number of function definitions match the previous number of function
        // declarations?
        for i in 0..num_func_defs {
            let must_interpret = i == callee_index;
            match self.decode_func_body(must_interpret, &mut cur) {
                Some(result) => match result {
                    EvalResult::I32(n) => println!("i32 result is {}", n),
                },
                None => {}
            }
        }
        Ok(())
    }

    fn call_func(&self, func_index: usize) -> Result<(), &'static str> {
        let mut cur = 0;
        if !self.check_header(&mut cur) {
            return Err("checking header failed");
        }

        let num_types = self.decode_type_section(&mut cur).unwrap();
        let num_decls = self.decode_func_decl(num_types, &mut cur);
        self.decode_exports_section(num_decls as u32, &mut cur);
        self.decode_code_section(func_index as u32, &mut cur);

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
    m.call_func(0);
}
