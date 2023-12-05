use std::collections::HashSet;

use lazy_static::lazy_static;
use swc_core::ecma::{
    ast::{
        Decl, ExportDecl, ExportNamedSpecifier, ExportSpecifier, Expr, ExprOrSpread, ExprStmt, Lit,
        ModuleExportName, ModuleItem, NamedExport, Pat, Stmt, Str, VarDeclarator,
    },
    visit::{Visit, VisitWith},
};

use crate::ExportInfo;

lazy_static! {
    static ref EXPORTS_SET: HashSet<&'static str> = HashSet::from([
        "getStaticProps",
        "getServerSideProps",
        "generateImageMetadata",
        "generateSitemaps",
        "generateStaticParams",
    ]);
}

pub(crate) struct CollectExportsVisitor {
    pub export_info: ExportInfo,
}

impl CollectExportsVisitor {
    pub fn new() -> Self {
        Self {
            export_info: Default::default(),
        }
    }
}

impl Visit for CollectExportsVisitor {
    fn visit_module_items(&mut self, stmts: &[swc_core::ecma::ast::ModuleItem]) {
        for stmt in stmts {
            if let ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                expr: box Expr::Lit(Lit::Str(Str { value, .. })),
                ..
            })) = stmt
            {
                if value == "use server" {
                    self.export_info.directives.insert("server".to_string());
                }
                if value == "use client" {
                    self.export_info.directives.insert("client".to_string());
                }
            }

            stmt.visit_children_with(self);
        }
    }

    fn visit_export_decl(&mut self, export_decl: &ExportDecl) {
        match &export_decl.decl {
            Decl::Var(box var_decl) => {
                if let Some(VarDeclarator {
                    name: Pat::Ident(name),
                    ..
                }) = var_decl.decls.first()
                {
                    if EXPORTS_SET.contains(&name.sym.as_str()) {
                        self.export_info.ssg = name.sym == "getStaticProps";
                        self.export_info.ssr = name.sym == "getServerSideProps";
                        self.export_info.generate_image_metadata =
                            Some(name.sym == "generateImageMetadata");
                        self.export_info.generate_sitemaps = Some(name.sym == "generateSitemaps");
                        self.export_info.generate_static_params =
                            name.sym == "generateStaticParams";
                    }
                }

                for decl in &var_decl.decls {
                    if let Pat::Ident(id) = &decl.name {
                        if id.sym == "runtime" {
                            self.export_info.runtime = decl.init.as_ref().and_then(|init| {
                                if let Expr::Lit(Lit::Str(Str { value, .. })) = &**init {
                                    Some(value.to_string())
                                } else {
                                    None
                                }
                            })
                        } else if id.sym == "preferredRegion" {
                            if let Some(init) = &decl.init {
                                if let Expr::Array(arr) = &**init {
                                    for expr in arr.elems.iter().flatten() {
                                        if let ExprOrSpread {
                                            expr: box Expr::Lit(Lit::Str(Str { value, .. })),
                                            ..
                                        } = expr
                                        {
                                            if let Some(preferred_region) =
                                                &mut self.export_info.preferred_region
                                            {
                                                preferred_region.push(value.to_string());
                                            }
                                        }
                                    }
                                } else if let Expr::Lit(Lit::Str(Str { value, .. })) = &**init {
                                    if let Some(preferred_region) =
                                        &mut self.export_info.preferred_region
                                    {
                                        preferred_region.push(value.to_string());
                                    }
                                }
                            }
                        } else {
                            self.export_info.extra_properties.insert(id.sym.to_string());
                        }
                    }
                }
            }
            Decl::Fn(fn_decl) => {
                let id = &fn_decl.ident;

                self.export_info.ssg = id.sym == "getStaticProps";
                self.export_info.ssr = id.sym == "getServerSideProps";
                self.export_info.generate_image_metadata = Some(id.sym == "generateImageMetadata");
                self.export_info.generate_sitemaps = Some(id.sym == "generateSitemaps");
                self.export_info.generate_static_params = id.sym == "generateStaticParams";
            }
            _ => {}
        }

        export_decl.visit_children_with(self);
    }

    fn visit_named_export(&mut self, named_export: &NamedExport) {
        for specifier in &named_export.specifiers {
            if let ExportSpecifier::Named(ExportNamedSpecifier {
                orig: ModuleExportName::Ident(value),
                ..
            }) = specifier
            {
                if !self.export_info.ssg && value.sym == "getStaticProps" {
                    self.export_info.ssg = true;
                }

                if !self.export_info.ssr && value.sym == "getServerSideProps" {
                    self.export_info.ssr = true;
                }

                if !self.export_info.generate_image_metadata.unwrap_or_default()
                    && value.sym == "generateImageMetadata"
                {
                    self.export_info.generate_image_metadata = Some(true);
                }

                if !self.export_info.generate_sitemaps.unwrap_or_default()
                    && value.sym == "generateSitemaps"
                {
                    self.export_info.generate_sitemaps = Some(true);
                }

                if !self.export_info.generate_static_params && value.sym == "generateStaticParams" {
                    self.export_info.generate_static_params = true;
                }

                if self.export_info.runtime.is_none() && value.sym == "runtime" {
                    self.export_info.warnings.push((
                        value.sym.to_string(),
                        "it was not assigned to a string literal".to_string(),
                    ));
                }

                if self.export_info.preferred_region.is_none() && value.sym == "preferredRegion" {
                    self.export_info.warnings.push((
                        value.sym.to_string(),
                        "it was not assigned to a string literal or an array of string literals"
                            .to_string(),
                    ));
                }
            }
        }

        named_export.visit_children_with(self);
    }
}