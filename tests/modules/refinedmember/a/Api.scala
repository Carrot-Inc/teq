package rma

// Members a refinement declares over the class's: selected by the refinement's denotation
// (dotty's `Types.findMember` `goRefined`), the call typed by its signature and pickled by it
// (`TreePickler.pickleTree`, `isRefinedMethod`).
trait B:
  def f[A](a: A): Any
  def g(x: Int, scale: Int = 10): Any

trait Ctx:
  type T
  val value: T

trait Runner:
  def run(c: Ctx): Any

class C extends B:
  def f[A](a: A): A = a
  def g(x: Int, scale: Int = 10): String = (x * scale).toString

object Api:
  type Refined = B { def f[A](a: A): A; def g(y: Int, scale: Int): String }
  def get: Refined = C()
  def twice(r: Refined): String = r.f[String]("ok") + r.g(y = 4)
  def read(k: Runner { def run(c: Ctx): c.T }, c: Ctx): c.T = k.run(c)
