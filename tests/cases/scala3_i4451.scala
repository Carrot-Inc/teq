// Copied from scala3 tests/run/i4451.scala (Apache-2.0, see tests/scala3/README.md).
// teq: --wconf msg=Ignoring..this..qualifier:s
import scala.annotation.threadUnsafe

class InitError extends Exception

class LazyThreadUnsafe(x: Int) {
  private[this] var init = false
  @threadUnsafe lazy val value = {
    if (!init) {
      init = true
      throw new InitError
    }
    x
  }
}

class Lazy(x: Int) {
  private[this] var init = false
  lazy val value = {
    if (!init) {
      init = true
      throw new InitError
    }
    x
  }
}

object Test {

  def tryTwice(x: => Int): Int =
    try {
      x; assert(false); 0
    } catch {
      case _: InitError => x
    }

  def lazyInMethod(x: Int) = {
    var init = false
    lazy val value = {
      if (!init) {
        init = true
        throw new InitError
      }
      x
    }
    tryTwice(value)
  }

  def main(args: Array[String]) = {
    val v = 42

    val l0 = new LazyThreadUnsafe(v)
    val l1 = new Lazy(v)
    assert(tryTwice(l0.value) == v)
    assert(tryTwice(l1.value) == v)
    assert(lazyInMethod(v) == v)
  }
}
