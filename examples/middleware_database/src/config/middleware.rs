use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet};
use gotham::pipeline::{new_pipeline, Pipeline};
use gotham_middleware_r2d2::R2D2Middleware;
use borrow_bag;
use r2d2::Pool;
use r2d2_redis::RedisConnectionManager;

type ConnMan = R2D2Middleware<RedisConnectionManager>;
type NewPipeline = Pipeline<(ConnMan, ())>;
type PipelineHandle = borrow_bag::Handle<NewPipeline, <() as borrow_bag::Append<NewPipeline>>::Navigator>;

pub fn make_pipelines() -> ((PipelineHandle, ()), PipelineSet<(NewPipeline, ())>) {
    let pipeline_set = new_pipeline_set();
    let redis_middleware = make_redis_connection_pool();
    let (pipelines, global) = pipeline_set.add(
        new_pipeline()
            .add(redis_middleware)
            .build(),
    );
    let default_pipeline_chain = (global, ());

    let final_pipeline_set = finalize_pipeline_set(pipelines);

    (default_pipeline_chain, final_pipeline_set)
}

fn make_redis_connection_pool() -> ConnMan {
    let manager = RedisConnectionManager::new("redis://localhost").unwrap();
    let pool = Pool::<RedisConnectionManager>::builder()
        .min_idle(Some(1))
        .build(manager)
        .unwrap();
    R2D2Middleware::with_pool(pool)
}
