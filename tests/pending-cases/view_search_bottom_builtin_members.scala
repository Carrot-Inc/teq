// `searchImplicit`'s `TooUnspecific` (Implicits.scala:1684-1686): a selection that finds no member on a qualifier of
// `Nothing` or `Null` (String's `toUpperCase`, StringOps' `toInt`, RichInt's `max` are members of no class of theirs)
// tries no view, so scalac 3.8.4 rejects every line as the qualifier's mismatch with TooUnspecific's note. teq's
// bootstrap stands for those members with package extensions it admits by the receiver's conformance, which `Nothing`
// and `Null` always have. On the head (61c1af7941) teq rejects the two `.foo` lines and `(??? : Nothing).length` (that
// with an ambiguity of two wrappers); on master (efddb38de3) the two `.foo` lines and `(null: Null).toUpperCase`, as
// "not a member": that line's acceptance on the head is TR's regression, the other lines are queued: push 6 (the
// bootstrap's part of `tryExtension`, Typer.scala:4274-4280: a builtin member a candidate only where the dispatch
// type's class derives from its owner, as `findMember` has it; typer_select.rs `try_lexical_extension`, apply.rs
// `is_builtin_member`; push 5's interim review, finding 1).
object Main:
  def main(args: Array[String]): Unit =
    println((??? : Nothing).toUpperCase)
    println((??? : Nothing).foo)
    println((??? : Nothing).length)
    println((??? : Nothing).toInt)
    println((null: Null).toUpperCase)
    println((null: Null).foo)
    println((??? : Nothing).max(1))
