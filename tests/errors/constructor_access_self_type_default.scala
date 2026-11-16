// expect: 9:9: error: illegal inheritance: self type C11 of class C11 does not conform to self type Mark of parent class SelfD
// expect: 1 error found
// A class refused for its parent's self type (scalac's E058, a line of its RefChecks) is not told of the parent
// call's default getter beside it, the line of a phase scalac's run does not reach then.
package lib:
  trait Mark
  class SelfD protected (val a: Int = 1) { self: Mark => }
package app:
  class C11 extends lib.SelfD
