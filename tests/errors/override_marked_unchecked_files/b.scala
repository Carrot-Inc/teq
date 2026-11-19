class C extends B with A
// expect: b.scala:1:7: error: class C inherits conflicting members
// expect: a.scala:9:81: error: error overriding type T in trait Types
// expect: 2 errors found
