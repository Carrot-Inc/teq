// teq: --werror
// A join of branches is annotated where `TypeComparer.lub` keeps an annotated operand
// (TypeAssigner.scala 429, 468, 479-481; TypeComparer.scala 2572-2614): two receivers' singletons
// are a union; a singleton that conforms to the other operand gives way to it (`opt.runtimeChecked`
// then `opt` keeps `opt`); a handler's plain `opt` after an annotated body likewise; of two types
// each conforming to the other the first stays (a plain ascription first). scalac's message for the
// two receivers names their union, `(opt2 : Option[Int]) | (opt : Option[Int])`, which its `lub`
// keeps of disjoint singletons (TypeComparer.scala 2593) and teq's widens to `Option[Int]`. A third
// branch conforming to a part of that union leaves it a union (`opt2` after `opt2.runtimeChecked`).
import scala.annotation.internal.RuntimeChecked
def f(c: Boolean, opt: Option[Int], opt2: Option[Int]): Unit =
  val Some(a) = if c then opt.runtimeChecked else opt2.runtimeChecked
  val Some(b) = if c then opt.runtimeChecked else opt
  val Some(d) = try opt.runtimeChecked catch { case _: Exception => opt }
  val Some(e) = if c then (opt2: Option[Int]) else (opt: Option[Int] @RuntimeChecked)
  val Some(g) = c match { case true => opt.runtimeChecked; case false => opt2.runtimeChecked }
  val Some(h) = (if c then 0 else 1) match { case 0 => opt.runtimeChecked; case 1 => opt2.runtimeChecked; case _ => opt2 }
  ()
// expect: 12:17: warning: pattern's type Some[Int]
// expect: 13:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 14:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 15:17: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 16:17: warning: pattern's type Some[Int]
// expect: 17:17: warning: pattern's type Some[Int]
