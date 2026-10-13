// A declaration `TypeOps.classBound` leaves out of an anonymous class's refinements (a nested class, a private
// member) is no member of the instance, and a refinement's type avoids it: `item`'s class is its class bound, the
// parent class, which has no `secret` (scalac says `Object`, teq `AnyRef`).
import scala.reflect.Selectable.reflectiveSelectable
val x = new reflect.Selectable { class Hidden { def secret: Int = 7 }; val item = new Hidden }
val leaked: Int = x.item.secret
val y = new reflect.Selectable { private val secret = 7; val public = 1 }
val leak = y.secret
// expect: anon_class_bound_avoid.scala:6:19: error: value secret is not a member of
// expect: anon_class_bound_avoid.scala:8:12: error: value secret is not a member of Selectable{val public: Int}
// expect: 2 errors found
