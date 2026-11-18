// A `try` whose handler is an expression is typed handler first, against `Throwable => pt`, the handler made its
// one case; then the body: the handler's `Unit` bounds the open variable, and the body's value is dropped against
// it. Each form of handler (a method, a lambda typed or not, a function value, a partial function value or
// literal, a generic factory, a block), with and without `finally`, over a body that returns or throws, as a
// lambda's result, a branch, a case and a by-name argument; a list of cases after a body of `Unit`.
def settle[B <: Unit](g: Int => B): B = g(0)
def settleByName[B <: Unit](b: => B): B = b
def tick(): Int = { println("effect"); 5 }
def boom(): Int = throw new RuntimeException("boom")
def handle(t: Throwable): Unit = println("handled " + t.getMessage)
val handler: Throwable => Unit = handle
val partial: PartialFunction[Throwable, Unit] = { case t => println("partial " + t.getMessage) }
def make[A](a: A): Throwable => A = t => { println("made " + t.getMessage); a }

object HandlerForms:
  def go(): Unit =
    settle(x => try tick() catch handle)
    settle(x => try { tick() } catch handle finally println("finally"))
    settle(x => try { if x > 0 then tick() else tick() } catch handle finally println("finally"))
    settle(x => try tick() catch ((t: Throwable) => println("lambda")))
    settle(x => try tick() catch (t => println("untyped")))
    settle(x => try tick() catch handler)
    settle(x => try tick() catch partial)
    settle(x => try tick() catch make(()))
    settle(x => try tick() catch { handle })
    settle(x => try tick() catch ({ case _: Exception => println("literal") }))
    println("forms done")

object HandlerThrown:
  def go(): Unit =
    settle(x => try boom() catch handle)
    settle(x => try boom() catch handler finally println("finally"))
    settle(x => try boom() catch partial)
    settle(x => try boom() catch make(()))
    settle(x => try throw new RuntimeException("thrown") catch handle)
    settle(x => try { if x > 0 then tick() else boom() } catch ({ case e: Exception => println("literal " + e.getMessage) }))
    println("thrown done")

object HandlerPlaces:
  def go(): Unit =
    settle(x => if x >= 0 then (try tick() catch handle) else ())
    settle(x => if x > 0 then () else (try boom() catch handle))
    settle(x => x match { case 0 => try tick() catch handle finally println("finally"); case _ => () })
    settleByName(try tick() catch handle)
    settleByName(try boom() catch partial)
    settle(x => try println("body") catch { case e: Exception => println("case " + e.getMessage) })
    println("places done")

@main def run(): Unit =
  HandlerForms.go()
  HandlerThrown.go()
  HandlerPlaces.go()
