package pac

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(new pab.D2); println("|")
