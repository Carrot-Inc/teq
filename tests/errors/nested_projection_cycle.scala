class EagerOuter {
  final val value: Inner#Bound = ???
  class Inner {
    type Bound <: value.type
  }
}
class LazyOuter {
  class Inner {
    type Bound <: value.type
  }
  lazy val value: Inner#Bound = ???
}
// expect: cyclic
// expect: 2 errors found
