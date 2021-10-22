use crdts::{list, CmRDT, List};
use std::fmt;

pub type Actor = usize;
pub type Gene = u8;
type ListOfGenes = List<Gene, Actor>;

pub struct Genome {
    genes: ListOfGenes,
}

impl Genome {
    fn new() -> Self {
        Genome {
            genes: ListOfGenes::new(),
        }
    }

    /// append appends an item to the genome,
    /// it returns an Op that can be passed to other actors
    /// probably serialized to json over http
    pub fn append(&mut self, item: u8, actor: Actor) -> list::Op<Gene, Actor> {
        let op = self.genes.append(item, actor);
        self.genes.apply(op.clone());
        op
    }

    /// apply applies an op, probably one created by a remote Actor
    pub fn apply(&mut self, op: list::Op<Gene, Actor>) {
        self.genes.apply(op);
        tracing::debug!("after Genome::apply: {}", self);
    }

    #[cfg(test)]
    fn is_equal(&self, rhs: &Self) -> bool {
        use std::cmp::Ordering;
        self.genes.iter().cmp(rhs.genes.iter()) == Ordering::Equal
    }
}

impl Default for Genome {
    fn default() -> Self {
        Genome::new()
    }
}

impl fmt::Display for Genome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = String::new();
        for gene in self.genes.iter() {
            let s = format!("{:02x}", gene);
            out.push_str(&s);
        }
        write!(f, "{}", out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_empty_genome() {
        let g = Genome::new();
        assert!(g.genes.is_empty());
        let v = g.genes.read::<Vec<&u8>>();
        assert!(v.is_empty());
    }

    #[test]
    fn can_append() {
        let mut g = Genome::new();
        assert!(g.genes.is_empty());

        let x: Gene = 42;
        let a: Actor = 111;

        let _op = g.append(x, a);

        let v = g.genes.read::<Vec<&u8>>();
        assert_eq!(v, vec![&42]);
    }

    #[test]
    fn append_preserves_order() {
        let mut g1 = Genome::new();
        let mut g2 = Genome::new();

        const A1: Actor = 111;
        const A2: Actor = 222;

        let mut g1ops = vec![g1.append(1, A1)];
        g1ops.push(g1.append(2, A1));
        g1ops.push(g1.append(3, A1));

        let mut g2ops = vec![g2.append(4, A2)];
        g2ops.push(g2.append(5, A2));
        g2ops.push(g2.append(6, A2));

        for op in &g2ops {
            g1.apply(op.clone());
        }

        for op in &g1ops {
            g2.apply(op.clone());
        }

        let g1_genes = g1.genes.read::<Vec<&u8>>();
        println!("g1 genes = {:?}", g1_genes);

        let g2_genes = g2.genes.read::<Vec<&u8>>();
        println!("g2 genes = {:?}", g2_genes);

        assert!(g1.is_equal(&g2));
    }

    #[test]
    fn display_looks_right() {
        const ACTOR: Actor = 42;
        let mut g = Genome::new();
        assert_eq!(format!("{}", g), "");
        g.apply(g.genes.append(0, ACTOR));
        assert_eq!(format!("{}", g), "00");
        g.apply(g.genes.append(1, ACTOR));
        assert_eq!(format!("{}", g), "0001");
        g.apply(g.genes.append(255, ACTOR));
        assert_eq!(format!("{}", g), "0001ff");
    }
}
