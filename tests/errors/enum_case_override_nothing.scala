// expect: 6:23: error: value t overrides nothing
// absent: 7:23: warning: cannot override val parameter
// An enum case's `override val` parameter that overrides no member of its enum is E037 (`OverridesNothing`) at
// every source version, apart from (1.12); one that overrides the enum's like-named val parameter is its accessor.
enum F(val s: Int):
  case G(override val t: Int) extends F(t)
  case H(override val s: Int) extends F(s)
