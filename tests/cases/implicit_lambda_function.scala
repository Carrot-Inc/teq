// An `implicit` lambda without a parameter type where a plain function is expected, an inline
// parameter's too: its parameter takes the expected function's argument type (`Typer.typedFunctionValue`
// infers it from `pt`, `ProtoTypes`) and is a given in the body, as zio's `Unsafe.unsafe { implicit u => .. }`
// is written; scalac 3.8.4 accepts each.
class U(val n: Int)
object Main:
  def use[A](f: U => A): A = f(new U(1))
  inline def useInline[A](inline f: U => A): A = f(new U(2))
  def main(args: Array[String]): Unit =
    println(use { implicit u => summon[U].n + 10 })
    println(useInline { implicit u => summon[U].n + 20 })
