package icallb

@main def run(): Unit =
  println("before")
  println(icalla.twice({ println("argument"); 2 }))
