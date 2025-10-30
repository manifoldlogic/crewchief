# Django urls.py example - URL configuration patterns
from django.urls import path, include
from django.contrib import admin
from django.conf import settings
from django.conf.urls.static import static
from django.views.generic import TemplateView

from . import views

app_name = 'shop'

# Product URL patterns
product_patterns = [
    path('', views.ProductListView.as_view(), name='product_list'),
    path('create/', views.ProductCreateView.as_view(), name='product_create'),
    path('<slug:slug>/', views.ProductDetailView.as_view(), name='product_detail'),
    path('<slug:slug>/edit/', views.ProductUpdateView.as_view(), name='product_edit'),
    path('<slug:slug>/review/', views.add_review, name='add_review'),
]

# Category URL patterns
category_patterns = [
    path('', views.CategoryListView.as_view(), name='category_list'),
]

# Order URL patterns
order_patterns = [
    path('', views.OrderListView.as_view(), name='order_list'),
    path('<int:pk>/', views.OrderDetailView.as_view(), name='order_detail'),
]

# API URL patterns
api_patterns = [
    path('search/', views.api_product_search, name='api_product_search'),
    path('products/', views.ProductAPIView.as_view(), name='api_products'),
]

# Main URL patterns
urlpatterns = [
    # Home
    path('', views.home, name='home'),

    # Admin
    path('admin/', admin.site.urls),

    # Products
    path('products/', include(product_patterns)),

    # Categories
    path('categories/', include(category_patterns)),

    # Orders
    path('orders/', include(order_patterns)),

    # API
    path('api/', include(api_patterns)),

    # Static pages
    path('about/', TemplateView.as_view(template_name='about.html'), name='about'),
    path('contact/', TemplateView.as_view(template_name='contact.html'), name='contact'),
]

# Serve media files in development
if settings.DEBUG:
    urlpatterns += static(settings.MEDIA_URL, document_root=settings.MEDIA_ROOT)
    urlpatterns += static(settings.STATIC_URL, document_root=settings.STATIC_ROOT)

# Custom error handlers
handler404 = 'shop.views.handler404'
handler500 = 'shop.views.handler500'
