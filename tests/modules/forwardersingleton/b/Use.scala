package fsb

import fsa.Counter

type Plain = Counter.plain.type

def size(xs: List[Counter.plain.type]): Int = xs.size
def length(x: Plain): Int = x.length
def count(xs: List[Counter.Inner.type]): Int = xs.size

@main def run(): Unit =
  println(size(Nil))
  println(length(Counter.plain.asInstanceOf[Plain]))
  println(count(Nil))
