package stall

// A trait that extends `App`, in the first module: a method of it reads `args`.
trait Tool extends App:
  def count: Int = args.length
