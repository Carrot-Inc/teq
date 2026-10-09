package retypeinit.lib

// A file whose one val is a literal, without an initialiser, until an edit of its body makes the
// val eager: the calls of its def in another module, not typed again, then run the initialiser
// first, and the value made from the def checks it itself.
def noisy(x: Int): Int =
  println("counted")
  x

val counted: Int = noisy(1)

def twice(n: Int): Int = n * 2
