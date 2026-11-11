object BrokenAliases {
  opaque type Loop = Loop
  def local(): Unit = {
    opaque type Hidden = Int
    ()
  }
}
// expect: illegal cyclic type reference
// expect: this modifier is not allowed here
// expect: 2 errors found
