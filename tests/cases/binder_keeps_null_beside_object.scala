// Binders outside the set whose simplification is known to be scalac's keep the type master gives them: a `Null`
// beside an object's type inside a type argument, under an intersection or through a singleton stays, so a `null`
// scrutinee fails the type test. scalac 3.8.4 prints `other` four times.
trait Named
case object Obj extends Named
val obj = Obj
case class Box[A](a: A)

def boxed(v: Box[Obj.type | Null], flag: Boolean): String = v match
  case x => x.a match
    case y => y match
      case _: Named if flag => "named"
      case _ => "other"

def boxedAt(v: Box[Obj.type | Null], flag: Boolean): String = v match
  case x @ Box(_) => x.a match
    case y => y match
      case _: Named if flag => "named"
      case _ => "other"

def intersected(v: (Obj.type & Named) | Null, flag: Boolean): String = v match
  case y => y match
    case _: Named if flag => "named"
    case _ => "other"

def singleton(v: obj.type | Null, flag: Boolean): String = v match
  case y => y match
    case _: Named if flag => "named"
    case _ => "other"

@main def run(): Unit =
  println(boxed(Box(null), true))
  println(boxedAt(Box(null), true))
  println(intersected(null, true))
  println(singleton(null, true))
