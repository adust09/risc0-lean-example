import Guest.Basic

@[export risc0_main]
def risc0_main (input : ByteArray) : ByteArray :=
  let str := String.fromUTF8! input
  let n := str.toNat!
  let result := fibonacci n
  let resultStr := toString result
  resultStr.toUTF8
