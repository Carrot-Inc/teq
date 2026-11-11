// A string constant folded from two halves of a surrogate pair is the pair (`ConstFold.foldStringOp`'s `+` over Java
// strings): the fold of `"\uD83D" + "\uDE00"` equals `"😀"` at compile time, as `fold_binop` joins the halves through
// text.rs's unit-aware join (`text::push_str`; table (c)'s S-45 on the M2e refold).
package string_constant_fold_surrogates

final val joined = "\uD83D" + "\uDE00"

inline def folded: String = inline if joined == "😀" then "folded to the pair" else "folded apart"

@main def main(): Unit =
  println(folded)
  println(joined.length)
  println(joined == "😀")
