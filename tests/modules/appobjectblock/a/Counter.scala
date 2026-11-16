package stall

// An object that extends `App`, in the first module: its body gives `delayedInit` a block that reads `args`.
object Desk:
  object Counter extends App:
    println("counter body")
    delayedInit { println("counter block " + args.length) }
