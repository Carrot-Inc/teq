// An inline method whose body fails the definition check reports its errors at the definition
// alone: a call of it is not expanded again and stands as a call of a method of the declared result
// type, or of the result type the definition inferred (an error where that is one), as scalac 3.8.4
// drops the inline flags of a definition whose typing failed (`PrepareInlineable`): `generic[Int]`
// is an `Int`, which a `String` does not take. A plain match of an inline body is
// checked where it is defined, once however many calls expand it (scalac warns there, called or
// not, on a program without errors). scalac reports the ten errors; the count pins that no call
// adds its own and that each mismatch is there.
// expect: value noSuch is not a member of Int
// expect: type mismatch: found Int, required String
// expect: match may not be exhaustive; missing: None
// expect: value noSuch is not a member of T
// expect: 10 errors found
inline def bad(x: Int): Int = x.noSuch
transparent inline def badTransparent(x: Int): Int = x.noSuch
inline def badInferred(x: Int) = x.noSuch
transparent inline def badBoth(x: Int) = x.noSuch
inline def partial(x: Option[Int]): Int = x match { case Some(v) => v }
inline def generic[T](x: T) = { x.noSuch; x }
transparent inline def genericBoth[T](x: T) = { x.noSuch; x }
val a = bad(1)
val b: String = bad(2)
val c: String = badTransparent(3)
val d: String = badInferred(4)
val e: String = badBoth(5)
val g = partial(Some(1)) + partial(Some(2))
val h: String = generic[Int](6)
val i: String = genericBoth[Int](7)
val j: Int = generic(8)
