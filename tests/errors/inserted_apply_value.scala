// expect: inserted_apply_value.scala:16:11: error: value apply in class Ops does not take parameters
// expect: inserted_apply_value.scala:17:11: error: value apply in object O does not take parameters
// expect: inserted_apply_value.scala:18:11: error: method apply in class Thunk does not take parameters
// expect: inserted_apply_value.scala:19:11: error: method apply in class D does not take parameters
// The `apply` inserted for the arguments of a value is marked and no second one goes in after it
// (dotty's `Typer.tryInsertApplyOrImplicit`, `isSyntheticApply`): an `apply` that is a value, or
// a method without parameters, does not take the arguments even where its result would (E050).
class Ops(val apply: Int => Int)
object O:
  val apply: Int => Int = _ + 1
class Thunk { def apply: Int => Int = _ + 2 }
class C { def apply(n: Int): Int = n }
class D { def apply: C = C() }
@main def run(): Unit =
  val ops = Ops(_ + 3)
  println(ops(2))
  println(O(2))
  println(Thunk()(2))
  println(D()(22))
