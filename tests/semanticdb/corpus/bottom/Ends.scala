class In[-A]
object Ends {
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  val k = map(new Context)(x => (??? : In[x.T @Mark]))
  val l = map(new Context)(x => (??? : In[x.T]))
  val m = map(new Context)(x => (??? : In[In[x.T @Mark] @Mark]))
  val n = map(new Context)(x => (??? : Box[In[x.T] @Mark]))
}
