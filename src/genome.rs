use crdts::{list, List, CmRDT};
use std::cmp::Ordering;

type Actor = usize;
type Gene = u8;
type ListOfGenes = List::<Gene, Actor>;  

pub struct Genome {
    genes: ListOfGenes,
}

impl Genome {
    fn new() -> Self {
        Genome{
            genes: ListOfGenes::new(),
        }
    }

    fn apply(&mut self, op: list::Op::<Gene, Actor>) {
        self.genes.apply(op)
    }

    fn is_equal(&self, rhs: &Self) -> bool {
        self.genes.iter().cmp(rhs.genes.iter()) == Ordering::Equal        
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

        let op = g.genes.append(x, a);
        g.apply(op);

        let v = g.genes.read::<Vec<&u8>>();
        assert_eq!(v, vec![&42]);
    }

    #[test]
    fn append_preserves_order() {
        let mut g1 = Genome::new();
        let mut g2 = Genome::new();

        const A1: Actor = 111;
        const A2: Actor = 222;

        let mut g1ops = vec![];
        {
            let op = g1.genes.append(1, A1);
            g1ops.push(op.clone());
            g1.apply(op);
        }
        {
            let op = g1.genes.append(2, A1);
            g1ops.push(op.clone());
            g1.apply(op);
        }
        {
            let op = g1.genes.append(3, A1);
            g1ops.push(op.clone());
            g1.apply(op);
        }

        let mut g2ops = vec![];
        {
            let op = g2.genes.append(4, A2);
            g2ops.push(op.clone());
            g2.apply(op);
        }
        {
            let op = g2.genes.append(5, A2);
            g2ops.push(op.clone());
            g2.apply(op);
        }
        {
            let op = g2.genes.append(6, A2);
            g2ops.push(op.clone());
            g2.apply(op);
        }

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
}
