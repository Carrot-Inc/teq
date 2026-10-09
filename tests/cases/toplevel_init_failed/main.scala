@main def main(): Unit =
  try println(fi.f)
  catch case e: Throwable => println(e.getClass.getSimpleName)
  try println(fi.f)
  catch case e: Throwable => println(e.getClass.getSimpleName)
