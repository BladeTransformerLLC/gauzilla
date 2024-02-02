# Gauzilla
A 3D Gaussian Splatting (3DGS) renderer written in Rust for platform-agnostic WebAssembly (WASM) with lock-free multithreading.
* Uses WebGL and CPU splat sorting (based on [splat](https://github.com/antimatter15/splat)) for high compatibility among web browsers
* Circumvents [WASM's limitations in multithreading](https://rustwasm.github.io/2018/10/24/multithreading-rust-and-wasm.html) via the use of the lock-free [bus](https://github.com/jonhoo/bus) mechanism
* Uses [rfd](https://github.com/PolyMeilex/rfd) to securely load a .ply or .splat file stored locally on the host machine
* Loads a .splat file asynchronously from a URL (CDN) without having to use async code in Rust


![Screenshot #1](images/gauzilla_01.png?raw=true "Screenshot #1")

![Screenshot #2](images/gauzilla_02.png?raw=true "Screenshot #2")


## Web Demo
See the [Gauzilla Web Examples](https://github.com/BladeTransformerLLC/gauzilla/tree/main/examples) in this repository.


## Introduction
The innovative rendering technique known as 3D Gaussian Splatting (3DGS) [1] represents a Machine Learning-oriented approach to 3D rendering, specifically designed for Novel View Synthesis (NVS). It facilitates the real-time photorealistic rendering of scenes reconstructed from images and videos captured using conventional smartphone cameras. Since its release in 2023 there has been a Cambrian explosion of 3DGS applications and extensions, such as 4DGS [2], D3GA [3], SLAM [4], SC-GS [5], GPS [6], GHA [7], etc (see a survey [9]).

One of the original ideas of splatting Gaussian ellipsoids onto the 2D screen as ellipses for rendering can be traced back in 2002 [8] and 3DGS uses almost the same approach for its efficient forward rendering on the GPU.

Briefly, a 3D elliptical Gaussian centered at a point p with a covariance matirx V is defined as:

![Eq.19 of [8]](images/eq19.png?raw=true "Eq.19 of [8]")

The key to splatting it onto the 2D screen is to compute the 3x3 covariance matrix in ray coordinates:

![Eq.31 of [8]](images/eq31.png?raw=true "Eq.31 of [8]")

where W is the view transformation (camera) matrix, V_k = R * S * S^T * R^T (covariance matrix factorized with a rotation matrix R and a scaling matrix S, both of which can be computed, respectively, from quaternions and scalers stored in the PLY file), and

![Eq.29 of [8]](images/eq29.png?raw=true "Eq.29 of [8]")

is a Jacobian or the partial derivative of the camera-ray mapping at the point t_k in camera space (i.e., affine approxmation of the perspective projection).

Once the covariance matrix in ray coordinates is obtained via the simple matrix multiplications above, one can compute from the top-left 2x2 component of the matrix the major and minor axes representing the size and orientation of Gaussian ellipses, which can subsequently be rendered on the GPU efficiently using Geometry Instancing.


## How to Build & Use
1. Install the nightly version of Rust: `rustup toolchain install nightly`
2. Install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) and [sfz](https://github.com/weihanglo/sfz)
3. Run `./build.sh sfz` and open the locally-served URL in a web browser
4. Open a PLY file formatted for 3DGS (eg. download the [official pre-trained models](https://repo-sam.inria.fr/fungraph/3d-gaussian-splatting/datasets/pretrained/models.zip)) or a .splat file (use [this script](https://github.com/antimatter15/splat/blob/main/convert.py) to convert from PLY)

#### Orbit Camera Controls:
```
Left mouse button   - Rotate view around target
Middle mouse button - Zoom in/out
Right mouse button  - Move left/right/up/down
```

#### Fly Camera Controls:
```
Left mouse button   - Change view direction (free-look)
Middle mouse button - Move forward/backward
Right mouse button  - Move left/right/up/down
```

## How to Deploy on Web
1. (Optional) Enable `async_splat_stream` feature in Cargo.toml
2. Run `./build.sh`
3. Enable [cross-origin isolation](https://developer.chrome.com/blog/enabling-shared-array-buffer/) on the server (cf. [Vercel deployment configuration](https://github.com/BladeTransformerLLC/gauzilla_vercel/blob/main/vercel.json))


## ToDo
* Optimize `Scene::sort()` and `Scene::generate_texture()` (eg. parallelize using [wasm-bindgen-rayon](https://github.com/GoogleChromeLabs/wasm-bindgen-rayon))
* Implement asynch progressive splat loading/rendering for web hosting
* Allow camera controls with keyboard
* Write a WebGPU render path (cf. [splatter](https://github.com/Lichtso/splatter))


## References
1. Kerbl et al., ["3D Gaussian Splatting for Real-Time Radiance Field Rendering,"](https://repo-sam.inria.fr/fungraph/3d-gaussian-splatting/) 2023.
2. Wu et al., ["4D Gaussian Splatting for Real-Time Dynamic Scene Rendering,"](https://guanjunwu.github.io/4dgs/) 2023.
3. Zielonka et al., ["D3GA - Drivable 3D Gaussian Avatars,"](https://zielon.github.io/d3ga/) 2023.
4. Matsuki et al., ["Gaussian Splatting SLAM,"](https://rmurai.co.uk/projects/GaussianSplattingSLAM/) 2023.
5. Huang et al., ["SC-GS: Sparse-Controlled Gaussian Splatting for Editable Dynamic Scenes,"](https://yihua7.github.io/SC-GS-web/) 2023.
6. Zheng et al., ["GPS-Gaussian: Generalizable Pixel-wise 3D Gaussian Splatting for Real-time Human Novel View Synthesis,"](https://shunyuanzheng.github.io/GPS-Gaussian) 2023.
7. Xu et al., ["Gaussian Head Avatar: Ultra High-fidelity Head Avatar via Dynamic Gaussians,"](https://yuelangx.github.io/gaussianheadavatar/) 2022.
8. Zwicker et al., ["EWA Splatting,"](https://vcg.seas.harvard.edu/publications/ewa-splatting) 2002.
9. Chen & Wang, ["A Survey on 3D Gaussian Splatting,"](https://arxiv.org/abs/2401.03890) 2024.
