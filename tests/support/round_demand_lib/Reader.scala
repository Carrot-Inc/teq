package rounddemand

// Bodies that hold a call of the library's own macro: each body is typed from the pickle when it is first asked
// for and its calls are expanded then, or kept where the macro's run has to wait for a file of the build.
object Reader:
  def depth: Int = Derived.derived
  // A plain inline call in one branch and the macro's call in the other.
  inline def ready: Int = 13
  def either(first: Boolean): Int = if first then ready else Derived.derived
  // A literal in one branch and the macro's call in the other.
  def settled(first: Boolean): Int = if first then 13 else Derived.derived
