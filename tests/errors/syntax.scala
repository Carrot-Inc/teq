// expect: expected 'then'

def f(x: Int): Int =
  if x > 0 x else throw new Exception("no")
