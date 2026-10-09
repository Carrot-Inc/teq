import Macros.message

@main def run(): Unit =
  println(message("comma").length)
  println(message("greeting"))
  println(message("tabbed key").replace("\t", "<tab>"))
  println(message("spaced"))
  println(message("plain").isEmpty)
  println(message("backslash"))
  println(message("colon:in=key"))
  println(message("last"))
  println(message("crlf"))
  println(message("dup"))
  println(s"${message("emoji").length} ${message("emoji").codePointAt(0)}")
  println(message("eof"))
