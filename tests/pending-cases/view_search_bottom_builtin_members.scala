// `searchImplicit`'s `TooUnspecific` (Implicits.scala:1684-1686):
// a selection that finds no member on a qualifier of
// `Nothing` or `Null` (String's `toUpperCase`, StringOps'
// `toInt`, RichInt's `max` are members of no class of
// theirs) tries no view, so scalac 3.8.4 rejects every line
// as the qualifier's mismatch with TooUnspecific's note.
// teq is to reject them so. Queued: push 6 (the bootstrap's
// part of `tryExtension`, Typer.scala:4274-4280: a builtin
// member a candidate only where the dispatch type's
// class derives from its owner, as `findMember` has it).
object Main:
  def main(args: Array[String]): Unit =
    println((??? : Nothing).toUpperCase)
    println((??? : Nothing).foo)
    println((??? : Nothing).length)
    println((??? : Nothing).toInt)
    println((null: Null).toUpperCase)
    println((null: Null).foo)
    println((??? : Nothing).max(1))
