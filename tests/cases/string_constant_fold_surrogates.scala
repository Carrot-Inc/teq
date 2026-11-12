// A string constant folded from two halves of a surrogate pair is the pair (`ConstFold.foldStringOp`'s `+` over Java
// strings): the fold of `"\uD83D" + "\uDE00"` equals `"😀"` at compile time in scalac 3.8.4;
// teq's fold is to give the pair too.
package string_constant_fold_surrogates

final val joined = "\uD83D" + "\uDE00"

inline def folded: String = inline if joined == "😀" then "folded to the pair" else "folded apart"

@main def main(): Unit =
  println(folded)
  println(joined.length)
  println(joined == "😀")
