// expect: export_value_type_conflict.scala:11:12: error: T is already defined as type T
// expect: export_value_type_conflict.scala:14:12: error: T is already defined as type T
// expect: export_value_type_conflict.scala:16:12: error: T is already defined as type T
// expect: 3 errors found
// A value export forwards a type member as an alias, which a type of the class's of the same
// name, its own or an earlier clause's forwarder, conflicts with (dotty's `checkNoConflict`).
class A:
  type T = Int
class B(x: A, y: A):
  export x._
  export y._
class C(x: A):
  type T = String
  export x._
class D(x: A):
  export x._
  type T = String
