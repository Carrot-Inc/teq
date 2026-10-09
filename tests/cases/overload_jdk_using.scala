// A method with a trailing implicit clause next to a JDK class's method of the same name and
// explicit parameters is an overload, not an override, as zio's `Ref.Atomic.unsafe` has it.
import java.util.concurrent.atomic.AtomicReference

final class Token

trait UnsafeAPI[A]:
  def get(implicit t: Token): A
  def set(a: A)(implicit t: Token): Unit
  def getAndSet(a: A)(implicit t: Token): A

object Main:
  def make[A](initial: A): UnsafeAPI[A] =
    new AtomicReference[A](initial) with UnsafeAPI[A]:
      def get(implicit t: Token): A = this.asInstanceOf[AtomicReference[A]].get()
      def set(a: A)(implicit t: Token): Unit = this.asInstanceOf[AtomicReference[A]].set(a)
      def getAndSet(a: A)(implicit t: Token): A = this.asInstanceOf[AtomicReference[A]].getAndSet(a)

  def main(args: Array[String]): Unit =
    given Token = new Token
    val r = make(1)
    r.set(5)
    println(r.get)
    println(r.getAndSet(7))
    println(r.get)
    val raw = make("a").asInstanceOf[AtomicReference[String]]
    raw.set("b")
    println(raw.get())
