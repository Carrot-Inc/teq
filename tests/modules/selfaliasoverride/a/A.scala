package saa

trait ROps[R <: RDF]:
  def name: String

trait RDF:
  rdf =>
  type R = rdf.type
  type S = this.type
  def rops: ROps[R]
  def me: S
