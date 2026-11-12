// The guard's second fence: a binder over a union of an object's type, a class and `Null` keeps the union as it
// is written (`holds_null_and_object` in the pattern typing), so the type tests on it are those master makes.
trait Named
case object Obj extends Named
def a(v: Obj.type | String | Null, flag: Boolean): String = v match
  case y => y match
    case _: Named if flag => "named"
    case s: String => "string " + s
    case _ => "other"
@main def run(): Unit =
  println(a(null, true))
  println(a(Obj, true))
  println(a("s", true))
