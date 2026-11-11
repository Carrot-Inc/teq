type Curried[A] = [B] =>> [C] =>> (A, B, C)
val value: Curried[Int][String][Boolean] = (1, "x", true)
@main def run(): Unit = println(value)
