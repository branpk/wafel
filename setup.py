import shutil
from setuptools import setup
from setuptools_rust import Binding, RustExtension

import wafel.config as config

config.init()

shutil.rmtree('build/lib', ignore_errors=True)

# TODO: Replace with cargo clean -p wafel_python --target <all targets>
shutil.rmtree('target/x86_64-pc-windows-msvc', ignore_errors=True)

setup(
    name="wafel",
    version=config.version_str("."),
    rust_extensions=[
        RustExtension(
            "wafel", path="wafel_python/Cargo.toml", debug=False, binding=Binding.PyO3
        )
    ],
    packages=["wafel"],
    package_dir={"wafel": "wafel_python"},
    package_data={"wafel": ["py.typed", "__init__.pyi"]},
    zip_safe=False,
)
