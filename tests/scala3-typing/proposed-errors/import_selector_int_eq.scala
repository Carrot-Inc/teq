// expect: value eq is not a member of Int
// scalac 3.8.4: -- [E008] Not Found Error: Test.scala:1:34 ... value eq is not a member of Int
// (`eq`, `ne`, `synchronized`, `notify` and `wait` are `AnyRef`'s members, not a value type's; teq's universal
// member list grants them to `Int`). Pinned for the queued part inlining-lila-folds.
object Test { val n = 1; import n.eq }
