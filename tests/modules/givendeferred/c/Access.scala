package gda

object Access:
  def read(i: gdb.QualifiedImpl): Int = i.q
