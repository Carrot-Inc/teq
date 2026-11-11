// The library tests/support/missing_dep/A.scala is compiled against, left out of the jar and of its users' class path.
package dep
class Dep { def v: Int = 41 }
