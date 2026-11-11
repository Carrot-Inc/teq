// A downstream of a/ as tests/modules builds it. An inline parameter's getter is an inline method of its own
// (Desugar.scala:429), whose body makeInlineable prepares: a private member it reads goes through the class's inline
// accessor (`inline$k`, PrepareInlineable.scala), so an expansion outside the class can read it. Up to integration
// 9a42d3b487 teq pickled the inline given's getter body as `k + 1`, and scalac 3.8.4 compiling b/ over teq's
// products reported E173 at X.scala:16:24 ("value k cannot be accessed as a member of (C_this : (c : lib7.C)) ...").
// Since 2cac6ef7a6 teq pickles `C.this.inline$lib7$C$$k + 1`, as it does `d`'s, and scalac compiles b/ and prints the
// .expected. In tests/modules' harness, whose scalac-b and scalac-run checks are that mode, every run prints it. What
// fails is the whole build's classes against the split build's (classes, subset, bundle): the whole build's
// expansions read `k` through the accessor (`lib7$C$$inline$k()`), the build over the products through `C.k()`.
// Its cause is not located. It moves to tests/modules with the change that makes the two builds one. Reviewer #10's
// [R3] of the workers exports helper.
import lib7.*
@main def run(): Unit =
  val c = C(3)
  import c.given
  println(summon[String])
  println(c.d)
  println(c.g)
