package rounddemand

// A body with a plain inline call in one branch and a call of the library's own macro in the other: typed from
// the pickle, each call is expanded again.
object Reader:
  inline def ready: Int = 13
  def either(first: Boolean): Int = if first then ready else Derived.derived
