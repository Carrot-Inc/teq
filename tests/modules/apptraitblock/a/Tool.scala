package stall

// A trait that extends `App`, in the first module: its body reads `args` and gives `delayedInit` a block, and
// a method of it reads `args`.
trait Tool extends App:
  println("tool init " + (args == null))
  delayedInit { println("tool block " + args.length) }
  def count: Int = args.length
