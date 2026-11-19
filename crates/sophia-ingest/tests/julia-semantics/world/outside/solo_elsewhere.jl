# Another package (here `Main`) adds a method to `World.solo`: Sophia,
# ingesting only `World`, cannot see it.
World.solo(x::String) = "s"
