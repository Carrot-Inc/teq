package scala

// scala-library's `DelayedInit` and `App`. Scala 3 passes no body to `delayedInit`: the body of an object that
// extends `App` is the object's initialiser, and `main`, which the object inherits, sets the arguments and
// runs the blocks the program itself gave `delayedInit`, in the order it gave them. The public and protected
// members are scala-library's. The blocks are kept in an array where scala-library keeps a `ListBuffer`. The
// trait's initialiser makes it, as scala-library's makes the buffer, so `main` or `delayedInit` called before
// that (from a superclass's constructor) fails as scala-library's does; and a block registered while `main` runs
// the blocks is refused with the exception scala-library's buffer throws for it.
// The Scala.js library's `App` reports nothing under the `scala.time` property, and neither does this one.
@deprecated("DelayedInit semantics can be surprising. Support for `App` will continue. See the release notes for more details: https://github.com/scala/scala/releases/tag/v2.11.0", "2.11.0")
trait DelayedInit:
  def delayedInit(x: => Unit): Unit

@deprecated("Support for trait App is deprecated in Scala 3. Please refer to https://docs.scala-lang.org/scala3/book/methods-main-methods.html.", "3.8.0")
trait App extends DelayedInit:
  /** The time the object was made at, in milliseconds. */
  final val executionStart: Long = java.lang.System.currentTimeMillis()

  /** The command line arguments: `null` until `main` runs, so in the object's body too. */
  protected final def args: Array[String] = _args

  private var _args: Array[String] = compiletime.uninitialized

  // The blocks given to `delayedInit`: the first `initCount` cells of `initCode`, in the order given.
  private var initCode: Array[() => Unit] = new Array[() => Unit](4)
  private var initCount: Int = 0

  @deprecated("the delayedInit mechanism will disappear", "2.11.0")
  override def delayedInit(body: => Unit): Unit =
    if initCount == initCode.length then
      val grown = new Array[() => Unit](initCount * 2)
      var i = 0
      while i < initCount do
        grown(i) = initCode(i)
        i += 1
      initCode = grown
    initCode(initCount) = () => body
    initCount += 1

  /** Sets the arguments and runs the blocks given to `delayedInit`. */
  final def main(args: Array[String]): Unit =
    this._args = args
    val code = initCode
    val n = initCount
    var i = 0
    // The array's length is read before a cell is: called before the initialiser above has made the array,
    // `main` fails there, as scala-library's fails on its buffer.
    while i < code.length && i < n do
      code(i)()
      // scala-library's `for` over its buffer: a block that registers another is a mutation during iteration.
      if initCount != n then throw new java.util.ConcurrentModificationException("mutation occurred during iteration")
      i += 1
