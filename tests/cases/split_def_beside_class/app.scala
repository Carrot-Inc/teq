package app

def Codec(x: Int): lib.Codec = new lib.Codec(x + 1)
def make(x: Int): lib.Codec = new lib.Codec(x)

export lib.Codec as LibCodec

@main def run(): Unit =
  println(Codec(1).x)
  println(make(5).x)
  println(LibCodec.zero.x)
  println(new LibCodec(7).x)
