trait ExceptionFactory:
  inline def safe(value: Int): Int | IllegalArgumentException =
    if value < 0 then IllegalArgumentException(s"$value neg") else value

  transparent inline def message(value: String): IllegalArgumentException =
    IllegalArgumentException(value)

  inline def empty(): IllegalArgumentException = IllegalArgumentException()

object Exceptions extends ExceptionFactory

@main def run(): Unit =
  println(Exceptions.safe(3))
  println(Exceptions.safe(-1).asInstanceOf[IllegalArgumentException].getMessage)
  println(Exceptions.message("transparent").getMessage)
  println(Exceptions.empty().getMessage == null)
  println(IllegalArgumentException("plain").getMessage)
