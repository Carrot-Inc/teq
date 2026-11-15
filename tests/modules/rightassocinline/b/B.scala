import probe.*
def left: Int = { print("l"); 1 }
def in: InlineBox = { print("r"); new InlineBox }
def bn: ByName = { print("r"); new ByName }
@main def run(): Unit =
  print("inline:"); println(left +: in)
  print("by-name:"); println(left +: bn)
