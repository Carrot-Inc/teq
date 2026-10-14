package p.use

import p.*

@tag("type")
enum ViaExport:
  case One

@p.json.tag("direct")
enum Direct:
  case Two

@main def run(): Unit =
  println(p.json.Reflect.tagOf[ViaExport])
  println(p.json.Reflect.tagOf[Direct])
