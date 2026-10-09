// `twice` overrides a method that is not inline, so its body is typed at its definition as
// well, `inline if` in it as a plain conditional. That body asks for the result type of
// `Plain.sign`, which is inferred: the body of `sign` is typed on that demand and is no
// inline method's, whichever body asked.
// expect: b_plain.scala:4:29: error: inline if can only be used in an inline method
// expect: 1 error found
package demandedretained

trait Doubles:
  def twice(x: Int): Int

class Impl extends Doubles:
  inline def twice(x: Int): Int = inline if x > 0 then Plain.sign(x) * 2 else 0

@main def run(): Unit = println(Impl().twice(3))
