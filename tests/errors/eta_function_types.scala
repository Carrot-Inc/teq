// teq: --source 3.8
// `e _` of a value that does not expand is refused unless its type is a function type as `defn.isFunctionType` decides
// (`Migrations.typedAsFunction`): `FunctionN`, `ContextFunctionN` or a polymorphic function type, not a class that
// extends a function type (`=:=`, `<:<`, a class of its own). A polymorphic function value is warned of alone.
class F extends (Int => Int) { def apply(x: Int): Int = x }
val ev: Int =:= Int = summon[Int =:= Int]
val sub: Int <:< Any = summon[Int <:< Any]
val fn: F = new F
val p = [A] => (a: A) => a
val f1 = ev _
val f2 = sub _
val f3 = fn _
val f4 = p _
// expect: 10:10: error: Only function types can be followed by _ but the current expression has type
// expect: 11:10: error: Only function types can be followed by _ but the current expression has type
// expect: 12:10: error: Only function types can be followed by _ but the current expression has type F
// expect: 13:10: warning: The trailing ` _` for eta-expansion is unnecessary;
// absent: 10:10: warning:
// absent: 11:10: warning:
// absent: 12:10: warning:
// absent: 13:10: error:
