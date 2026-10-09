@main def run(): Unit =
  var i = 0
  while
    i < 3
  do
    i += 1
  end while
  for
    a <- Seq(1)
  do println(a)
  end for
  println(i)
