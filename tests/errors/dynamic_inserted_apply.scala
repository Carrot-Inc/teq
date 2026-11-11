// expect: 18:11: error: method apply in trait ContextFunction1 does not take more parameters
// expect: error: method applyDynamic in class Loop does not take more parameters
// expect: 2 errors found
import scala.language.dynamics

class Plain extends Dynamic:
  def applyDynamic(name: String)(): DummyImplicit ?=> Target = Target()

class Loop extends Dynamic:
  def applyDynamic(arg: Any): Loop = this

class Target extends Dynamic:
  def applyDynamic(name: String)(x: Int): Int = x

@main def run(): Unit =
  val plain = Plain()
  val loop = Loop()
  println(plain.go()(42))
  println(loop.go(43))
