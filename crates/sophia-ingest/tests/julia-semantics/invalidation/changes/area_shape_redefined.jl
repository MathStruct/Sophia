# Replaces `area(s::Shape)` from another file: the `invoke` call reaches the new method.
Inval.area(s::Inval.Shape) = 5.0
