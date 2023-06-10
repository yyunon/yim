use std::{
    cell::Ref,
    cell::RefCell,
    rc::{Rc, Weak},
};

pub mod OpStack {
    use super::Operator;
    use std::{
        cell::Ref,
        cell::RefCell,
        rc::{Rc, Weak},
    };
    pub fn NewOp(op_name: &str, func: fn() -> bool) -> Rc<RefCell<Operator>> {
        Rc::new(RefCell::new(Operator::new(op_name.to_string(), func)))
    }
    pub fn BiDirectionalLink(op1: &Rc<RefCell<Operator>>, op2: &Rc<RefCell<Operator>>) {
        RefCell::borrow_mut(op1).reciprocal_to = Some(Rc::downgrade(op2));
        RefCell::borrow_mut(op2).reciprocal_to = Some(Rc::downgrade(op1));
    }
    pub fn DirectionalLink(op1: &Rc<RefCell<Operator>>, op2: &Rc<RefCell<Operator>>) {
        RefCell::borrow_mut(op1).reciprocal_to = Some(Rc::downgrade(op2));
    }
    pub fn ShowRelationship(op1: &Rc<RefCell<Operator>>, op2: &Rc<RefCell<Operator>>) {
        RefCell::borrow(op1).show_reciprocal();
        RefCell::borrow(op2).show_reciprocal();
    }
    pub fn Borrow(op1: &Rc<RefCell<Operator>>) -> Ref<Operator> {
        RefCell::borrow(op1)
    }
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub signature: String,
    pub reciprocal_to: Option<Weak<RefCell<Operator>>>,
    pub ftor: fn() -> bool,
}

impl Operator {
    pub(crate) fn new(signature: String, function: fn() -> bool) -> Self {
        Self {
            signature: signature,
            reciprocal_to: None,
            ftor: function,
        }
    }

    pub(crate) fn show_reciprocal(&self) {
        println!(
            "{:?} is reciprocal to {:?}",
            self.signature,
            self.reciprocal_to
                .as_ref()
                .map(|s| Weak::upgrade(s).unwrap())
                .map(|s| RefCell::borrow(&s).signature.clone())
        );
    }

    pub(crate) fn run(&self) -> bool {
        let x = self.ftor;
        x()
    }
}

// These two implementations delegate to the Fn one.
// Of course, they might also be completely separate, if you like.
impl FnOnce<(String,)> for Operator {
    type Output = bool;
    extern "rust-call" fn call_once(self, args: (String,)) -> bool {
        self.call(args)
    }
}
impl FnMut<(String,)> for Operator {
    extern "rust-call" fn call_mut(&mut self, args: (String,)) -> bool {
        self.call(args)
    }
}
impl Fn<(String,)> for Operator {
    extern "rust-call" fn call(&self, args: (String,)) -> bool {
        self.run()
    }
}
