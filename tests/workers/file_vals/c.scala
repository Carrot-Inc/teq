package p

class Reader extends Source:
  def read(): Int = total

inline implicit def reader: Source = new Reader
