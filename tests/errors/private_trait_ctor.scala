// expect: 8:23: error: the constructor of T is private to T
// expect: 9:16: error: the constructor of T is private to T
// expect: 2 errors found
object o:
  trait T private[o]()
  class Inside extends T

class Outside extends o.T
def anon = new o.T { }
