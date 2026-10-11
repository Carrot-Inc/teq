package sab

import saa.*

object Model extends RDF:
  override def rops: ROps[R] = new ROps[R]:
    def name = "model"
  def me: S = this

class Graph extends RDF:
  def rops: ROps[R] = new ROps[R]:
    def name = "graph"
  def me: S = this
