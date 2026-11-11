package rpc

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(new rpb.D5); println("|")
  ignore(new rpb.D6); println("|")
  ignore(new rpb.D7); println("|")
